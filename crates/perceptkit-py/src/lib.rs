//! perceptkit Python bindings.
//!
//! Exposes:
//! - `version()`, `core_version()`, `audio_version()`
//! - `SceneEngine` class with `from_dir(path)`, `analyze_audio(np, sr)`,
//!   `analyze_bundle(dict)`, `scenes()`, `lint(path)` (staticmethod)
//! - `SceneDecision` dataclass-like with `scene_id / confidence /
//!   description / source / rationale`

use std::path::Path;

use numpy::PyReadonlyArray1;
use perceptkit_audio::AudioProvider;
use perceptkit_core::{
    FeatureBundle, FeatureKey, FeatureValue, SceneDecision as CoreDecision,
    SceneEngine as CoreEngine,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ── free functions ────────────────────────────────────────────────────

#[pyfunction]
fn core_version() -> &'static str {
    perceptkit_core::VERSION
}

#[pyfunction]
fn audio_version() -> &'static str {
    perceptkit_audio::VERSION
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ── SceneDecision ─────────────────────────────────────────────────────

#[pyclass(name = "SceneDecision", frozen)]
struct PyDecision {
    #[pyo3(get)]
    scene_id: Option<String>,
    #[pyo3(get)]
    confidence: f64,
    #[pyo3(get)]
    description: Option<String>,
    #[pyo3(get)]
    source: String,
    #[pyo3(get)]
    rationale: Vec<String>,
}

#[pymethods]
impl PyDecision {
    fn __repr__(&self) -> String {
        format!(
            "SceneDecision(scene_id={:?}, confidence={:.4}, source='{}')",
            self.scene_id, self.confidence, self.source
        )
    }

    fn is_known(&self) -> bool {
        self.scene_id.is_some()
    }
}

impl From<CoreDecision> for PyDecision {
    fn from(d: CoreDecision) -> Self {
        let source = format!("{:?}", d.source).to_lowercase();
        Self {
            scene_id: d.scene_id,
            confidence: d.confidence,
            description: d.description,
            source,
            rationale: d.rationale.into_iter().map(|e| e.description).collect(),
        }
    }
}

// ── SceneEngine ───────────────────────────────────────────────────────

#[pyclass(name = "SceneEngine", unsendable)]
struct PyEngine {
    engine: CoreEngine,
    audio_provider: AudioProvider,
}

#[pymethods]
impl PyEngine {
    /// Load scenes from a YAML directory. Defaults wire:
    /// - SimpleRuleMatcher / PriorityArbiter / ThresholdGate / NoopReflector
    /// - AudioProvider with Energy + VAD + MultiSpeaker extractors
    #[staticmethod]
    fn from_dir(path: &str) -> PyResult<Self> {
        let engine = CoreEngine::from_dir(Path::new(path))
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(Self {
            engine,
            audio_provider: AudioProvider::with_defaults(),
        })
    }

    /// Same as `from_dir` but swaps the heuristic VAD for Silero (rten
    /// backend). `model_path` must point to a `.rten` file (convert via
    /// `pip install rten-convert; rten-convert silero_vad.onnx silero_vad.rten`).
    /// Only available when the `silero-vad` build feature is enabled.
    #[cfg(feature = "silero-vad")]
    #[staticmethod]
    fn from_dir_silero(scenes_path: &str, model_path: &str) -> PyResult<Self> {
        let engine = CoreEngine::from_dir(Path::new(scenes_path))
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        let provider = AudioProvider::with_defaults_silero(Path::new(model_path))
            .map_err(|e| PyValueError::new_err(format!("silero load: {e}")))?;
        Ok(Self {
            engine,
            audio_provider: provider,
        })
    }

    /// Analyze raw PCM (f32 mono in [-1, 1]) at `sample_rate` Hz → SceneDecision.
    ///
    /// Zero-copy into Rust: expects C-contiguous numpy.ndarray (dtype=float32).
    fn analyze_audio(
        &self,
        pcm: PyReadonlyArray1<'_, f32>,
        sample_rate: u32,
    ) -> PyResult<PyDecision> {
        let slice = pcm
            .as_slice()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let bundle = self.audio_provider.process(slice, sample_rate, ts);
        let decision = self.engine.evaluate(&bundle);
        Ok(PyDecision::from(decision))
    }

    /// Extract audio features from PCM → dict (bool/float/str values).
    /// Useful for benches that want to merge extracted features with
    /// synthetic context before calling `analyze_bundle`.
    fn extract_audio_features<'py>(
        &self,
        py: Python<'py>,
        pcm: PyReadonlyArray1<'_, f32>,
        sample_rate: u32,
    ) -> PyResult<Bound<'py, PyDict>> {
        let slice = pcm
            .as_slice()
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let bundle = self.audio_provider.process(slice, sample_rate, ts);
        let d = PyDict::new_bound(py);
        for (k, v) in bundle.iter() {
            match v {
                FeatureValue::F64(x) => d.set_item(k.as_str(), *x)?,
                FeatureValue::Bool(x) => d.set_item(k.as_str(), *x)?,
                FeatureValue::Category(s) => d.set_item(k.as_str(), s.as_str())?,
                FeatureValue::Vector(v) => d.set_item(k.as_str(), v.clone())?,
            }
        }
        Ok(d)
    }

    /// Analyze a feature dict (keys like "audio.voice_ratio", values bool/f64/str).
    fn analyze_bundle(&self, features: &Bound<'_, PyDict>) -> PyResult<PyDecision> {
        let mut bundle = FeatureBundle::new(0.0);
        for (k, v) in features.iter() {
            let key_str: String = k
                .extract()
                .map_err(|_| PyValueError::new_err("feature keys must be strings"))?;
            let key =
                FeatureKey::new(&key_str).map_err(|e| PyValueError::new_err(format!("{e}")))?;
            let value = py_to_feature_value(&v)
                .map_err(|e| PyValueError::new_err(format!("feature '{key_str}': {e}")))?;
            bundle.insert(key, value);
        }
        let decision = self.engine.evaluate(&bundle);
        Ok(PyDecision::from(decision))
    }

    /// Names of loaded scenes.
    fn scene_ids(&self) -> Vec<String> {
        self.engine.scenes().iter().map(|s| s.id.clone()).collect()
    }

    /// Number of loaded scenes.
    fn scene_count(&self) -> usize {
        self.engine.scenes().len()
    }

    /// Installed audio extractor names (for audit).
    fn extractor_names(&self) -> Vec<String> {
        self.audio_provider
            .extractor_names()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Lint a scenes directory — reports scenes_ok count and conflicts.
    /// Returns dict: {scenes_ok, conflicts: [(a,b,reason)], warnings}.
    #[staticmethod]
    fn lint<'py>(py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyDict>> {
        let report =
            CoreEngine::lint(Path::new(path)).map_err(|e| PyValueError::new_err(format!("{e}")))?;
        let d = PyDict::new_bound(py);
        d.set_item("scenes_ok", report.scenes_ok)?;
        d.set_item(
            "conflicts",
            report
                .conflicts
                .iter()
                .map(|c| (c.scene_a.clone(), c.scene_b.clone(), c.reason.clone()))
                .collect::<Vec<_>>(),
        )?;
        d.set_item("warnings", report.warnings.clone())?;
        d.set_item("passed", report.passed())?;
        Ok(d)
    }
}

fn py_to_feature_value(v: &Bound<'_, PyAny>) -> Result<FeatureValue, String> {
    if let Ok(b) = v.extract::<bool>() {
        return Ok(FeatureValue::Bool(b));
    }
    if let Ok(f) = v.extract::<f64>() {
        return Ok(FeatureValue::F64(f));
    }
    if let Ok(s) = v.extract::<String>() {
        return Ok(FeatureValue::Category(s));
    }
    Err("unsupported Python type; expected bool / float / str".into())
}

// ── module entry ──────────────────────────────────────────────────────

#[pymodule]
fn _perceptkit(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(core_version, m)?)?;
    m.add_function(wrap_pyfunction!(audio_version, m)?)?;
    m.add("__version__", version())?;
    m.add_class::<PyDecision>()?;
    m.add_class::<PyEngine>()?;
    Ok(())
}
