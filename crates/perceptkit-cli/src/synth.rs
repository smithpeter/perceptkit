//! `perceptkit synthesize` — generate a labeled synthetic JSONL dataset for
//! CI evaluation gates.
//!
//! Produces deterministic feature bundles per scene using seeded randomness.
//! Each scene has a "template" of feature values with noise injected.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use serde::Serialize;

/// Simple deterministic PRNG (xorshift*).
struct Prng(u64);
impl Prng {
    fn new(seed: u64) -> Self {
        Self(
            seed.wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407),
        )
    }
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((v >> 32) as u32 as f64) / (u32::MAX as f64)
    }
    fn noise(&mut self, scale: f64) -> f64 {
        (self.next_f64() - 0.5) * scale * 2.0
    }
}

#[derive(Serialize)]
struct Row {
    features: BTreeMap<String, serde_json::Value>,
    label: String,
}

struct SceneTemplate {
    label: &'static str,
    features: &'static [(&'static str, FeatureTemplate)],
}

enum FeatureTemplate {
    F64(f64, f64), // (mean, noise_scale)
    Category(&'static [&'static str]),
}

// Templates match scenes/*.yaml semantics.
static TEMPLATES: &[SceneTemplate] = &[
    SceneTemplate {
        label: "office_quiet",
        features: &[
            ("audio.voice_ratio", FeatureTemplate::F64(0.10, 0.08)),
            ("audio.rms_db", FeatureTemplate::F64(-45.0, 4.0)),
        ],
    },
    SceneTemplate {
        label: "online_meeting",
        features: &[
            ("audio.voice_ratio", FeatureTemplate::F64(0.70, 0.10)),
            ("audio.rms_db", FeatureTemplate::F64(-25.0, 5.0)),
            (
                "context.app",
                FeatureTemplate::Category(&["Zoom", "Teams", "Feishu"]),
            ),
            ("audio.speaker_count", FeatureTemplate::F64(3.0, 0.5)),
        ],
    },
    SceneTemplate {
        label: "driving",
        features: &[
            ("audio.rms_db", FeatureTemplate::F64(-18.0, 4.0)),
            ("audio.voice_ratio", FeatureTemplate::F64(0.15, 0.10)),
            ("context.motion", FeatureTemplate::Category(&["vehicle"])),
        ],
    },
    SceneTemplate {
        label: "outdoor_noisy",
        features: &[
            ("audio.rms_db", FeatureTemplate::F64(-15.0, 3.0)),
            ("audio.voice_ratio", FeatureTemplate::F64(0.10, 0.08)),
        ],
    },
    SceneTemplate {
        label: "multi_speaker_chat",
        features: &[
            ("audio.voice_ratio", FeatureTemplate::F64(0.80, 0.08)),
            ("audio.rms_db", FeatureTemplate::F64(-20.0, 4.0)),
            ("audio.speaker_count", FeatureTemplate::F64(3.0, 0.5)),
            ("context.app", FeatureTemplate::Category(&["Messages"])),
        ],
    },
];

/// Synthesize a dataset to `out`, with `per_scene` rows per template and
/// base seed `seed`. Total rows = `TEMPLATES.len() * per_scene`.
pub fn synthesize_cmd(out: &Path, per_scene: usize, seed: u64) -> Result<ExitCode> {
    let file = File::create(out).with_context(|| format!("creating {}", out.display()))?;
    let mut writer = BufWriter::new(file);

    for (scene_idx, template) in TEMPLATES.iter().enumerate() {
        let mut prng = Prng::new(seed.wrapping_add(scene_idx as u64 * 1_000_000));
        for _ in 0..per_scene {
            let mut features = BTreeMap::new();
            for (key, tmpl) in template.features {
                let v = match tmpl {
                    FeatureTemplate::F64(mean, noise) => {
                        serde_json::Value::from(mean + prng.noise(*noise))
                    }
                    FeatureTemplate::Category(options) => {
                        let idx = (prng.next_f64() * options.len() as f64) as usize;
                        let idx = idx.min(options.len() - 1);
                        serde_json::Value::String(options[idx].into())
                    }
                };
                features.insert((*key).to_string(), v);
            }
            let row = Row {
                features,
                label: template.label.into(),
            };
            serde_json::to_writer(&mut writer, &row)?;
            writer.write_all(b"\n")?;
        }
    }

    writer.flush()?;
    let total = TEMPLATES.len() * per_scene;
    println!(
        "synthesized {} rows across {} scenes → {}",
        total,
        TEMPLATES.len(),
        out.display()
    );
    Ok(ExitCode::SUCCESS)
}
