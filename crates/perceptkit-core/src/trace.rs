//! ReflectionTrace — replayable JSONL audit trail for Reflector calls.
//!
//! STRATEGY §4.C: "Every Reflection comes with a replayable trace." This
//! module defines the on-disk format and a `JsonlTracer` implementation that
//! SceneEngine can attach for audit.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::reflector::Reflection;

/// One audit record per Reflector call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionTrace {
    /// Unix seconds when the reflection was logged.
    pub logged_at: f64,
    /// Case id from the PendingCase that triggered reflection.
    pub case_id: String,
    /// Bundle timestamp (not log timestamp).
    pub bundle_timestamp: f64,
    /// Reflector backend name (e.g. "noop", "mock", "local").
    pub reflector_name: String,
    /// Stable reflector fingerprint (for snapshot comparison).
    pub reflector_fingerprint: String,
    /// Input feature summary (key → display string).
    pub input_features: Vec<(String, String)>,
    /// Why hot path escalated.
    pub escalation_reason: String,
    /// Output reflection.
    pub output: Reflection,
    /// Wall-clock duration of the reflect() call in milliseconds.
    pub duration_ms: u64,
}

/// Trait implemented by anything that can sink `ReflectionTrace` records.
pub trait Tracer: Send + Sync {
    /// Record a trace. Implementations decide persistence vs silent drop.
    fn record(&self, trace: ReflectionTrace);
}

/// `Tracer` that appends one JSON record per line to a file.
pub struct JsonlTracer {
    path: PathBuf,
    // Mutex serializes writes — for M6+ high-concurrency, swap to channel.
    lock: Mutex<()>,
}

impl JsonlTracer {
    /// Create a tracer at `path` (file created if missing, appended if exists).
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    /// Underlying path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Tracer for JsonlTracer {
    fn record(&self, trace: ReflectionTrace) {
        let _guard = self.lock.lock();
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            if let Ok(json) = serde_json::to_string(&trace) {
                let _ = writeln!(file, "{json}");
            }
        }
    }
}

/// Read all traces from a JSONL file.
pub fn read_traces(path: &Path) -> Result<Vec<ReflectionTrace>> {
    let file = std::fs::File::open(path).map_err(|source| Error::Io {
        path: Some(path.to_path_buf()),
        source,
    })?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let t: ReflectionTrace = serde_json::from_str(trimmed)
            .map_err(|e| Error::Config(format!("trace line {}: {e}", i + 1)))?;
        out.push(t);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflector::Reflection;

    fn sample() -> ReflectionTrace {
        ReflectionTrace {
            logged_at: 1000.0,
            case_id: "c1".into(),
            bundle_timestamp: 999.5,
            reflector_name: "mock".into(),
            reflector_fingerprint: "mock@v1".into(),
            input_features: vec![("audio.voice_ratio".into(), "0.72".into())],
            escalation_reason: "low conf".into(),
            output: Reflection::unknown("test"),
            duration_ms: 5,
        }
    }

    #[test]
    fn jsonl_round_trip() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let tracer = JsonlTracer::new(tmp.path().to_path_buf());
        tracer.record(sample());
        tracer.record(sample());
        let traces = read_traces(tmp.path()).unwrap();
        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0].case_id, "c1");
    }

    #[test]
    fn read_skips_empty_and_comments() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        {
            let mut f = std::fs::File::create(tmp.path()).unwrap();
            writeln!(f, "# comment").unwrap();
            writeln!(f).unwrap();
            writeln!(f, "{}", serde_json::to_string(&sample()).unwrap()).unwrap();
        }
        let traces = read_traces(tmp.path()).unwrap();
        assert_eq!(traces.len(), 1);
    }
}
