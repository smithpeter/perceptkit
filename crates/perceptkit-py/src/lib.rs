//! perceptkit Python bindings via PyO3.
//!
//! This crate produces the `_perceptkit` cdylib that gets imported as
//! `perceptkit._perceptkit` from Python.
//!
//! Public API (v0.1 M1 scaffold):
//! - `version() -> str`
//!
//! Real API (M4+):
//! - `SceneEngine.from_dir(path)`
//! - `SceneEngine.analyze_audio(ndarray, sample_rate)`

use pyo3::prelude::*;

/// Return the perceptkit-core version string.
#[pyfunction]
fn core_version() -> &'static str {
    perceptkit_core::VERSION
}

/// Return the perceptkit-audio version string.
#[pyfunction]
fn audio_version() -> &'static str {
    perceptkit_audio::VERSION
}

/// Return the perceptkit package version.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _perceptkit(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(core_version, m)?)?;
    m.add_function(wrap_pyfunction!(audio_version, m)?)?;
    m.add("__version__", version())?;
    Ok(())
}
