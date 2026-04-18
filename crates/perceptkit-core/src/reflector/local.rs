//! LocalReflector — Qwen-0.5B Q4 via llama.cpp (Metal on Apple Silicon).
//!
//! Feature-gated by `local-reflector`. Requires a GGUF model file at a path
//! you configure at runtime. v0.1 is tested against Qwen2.5-0.5B-Instruct-Q4_K_M.
//!
//! Signal Model (DATA §2): model weights are on disk, no network calls.
//! llama.cpp is a compute dependency, not a network dependency.
//!
//! Usage:
//! ```no_run
//! # #[cfg(feature = "local-reflector")]
//! # {
//! use perceptkit_core::reflector::LocalReflector;
//! let reflector = LocalReflector::from_model_path(
//!     "./models/qwen2.5-0.5b-instruct-q4_k_m.gguf"
//! ).unwrap();
//! # }
//! ```

#![cfg(feature = "local-reflector")]
#![allow(deprecated)]

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;

use super::{PendingCase, PromptHash, ReflectError, Reflection, Reflector};

/// LocalReflector configuration.
#[derive(Debug, Clone)]
pub struct LocalConfig {
    /// Context length in tokens.
    pub n_ctx: u32,
    /// Number of layers to offload to the GPU (Metal on macOS).
    /// -1 (default in llama.cpp) offloads all; set to 0 for CPU only.
    pub n_gpu_layers: i32,
    /// Max new tokens per reflection.
    pub max_new_tokens: u32,
    /// Known scene ids (fed into the prompt so Qwen can Map to one).
    pub known_scenes: Vec<String>,
    /// System prompt prefix.
    pub system_prompt: String,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            n_ctx: 2048,
            n_gpu_layers: -1,
            max_new_tokens: 256,
            known_scenes: Vec::new(),
            system_prompt: default_system_prompt().into(),
        }
    }
}

fn default_system_prompt() -> &'static str {
    "You are a scene classifier for an AI agent. Given feature readings, pick one \
     of the known scenes if it fits, propose a new YAML scene if none fit, or say \
     Unknown. Respond with ONE line of valid JSON, no commentary.\n\
     Output shape is one of:\n\
       {\"kind\":\"map\",\"scene_id\":\"<id>\",\"rationale\":\"<short>\"}\n\
       {\"kind\":\"propose\",\"yaml\":\"<yaml>\",\"reason\":\"<why new>\"}\n\
       {\"kind\":\"unknown\",\"summary\":\"<one-sentence>\",\"top_features\":[\"<key>\",\"<key>\"]}"
}

/// Local Qwen-based reflector. Loads model once at construction, creates a
/// fresh context per reflect() call.
pub struct LocalReflector {
    model: Arc<LlamaModel>,
    backend: Arc<LlamaBackend>,
    model_path: PathBuf,
    config: LocalConfig,
}

impl LocalReflector {
    /// Load a GGUF model with default config.
    pub fn from_model_path(path: impl AsRef<Path>) -> Result<Self, ReflectError> {
        Self::with_config(path, LocalConfig::default())
    }

    /// Load a GGUF model with custom config.
    pub fn with_config(
        path: impl AsRef<Path>,
        mut config: LocalConfig,
    ) -> Result<Self, ReflectError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ReflectError::Backend(format!(
                "model file not found: {}",
                path.display()
            )));
        }

        // Backend init is idempotent on double-call (returns BackendAlreadyInitialized
        // which we ignore). We keep the Arc so drops are correctly ordered.
        let backend = Arc::new(
            LlamaBackend::init()
                .map_err(|e| ReflectError::Backend(format!("llama backend init: {e}")))?,
        );

        let mut model_params = LlamaModelParams::default();
        if config.n_gpu_layers != -1 {
            model_params = model_params.with_n_gpu_layers(config.n_gpu_layers.max(0) as u32);
        }

        let model = LlamaModel::load_from_file(&backend, path, &model_params)
            .map_err(|e| ReflectError::Backend(format!("load model: {e}")))?;

        // If no scenes pre-registered, leave empty — reflect() will still work
        // but will never Map (always Propose or Unknown).
        if config.known_scenes.is_empty() {
            tracing::warn!(
                "LocalReflector constructed with empty known_scenes; Reflection::Map is unreachable"
            );
        }

        // Ensure known_scenes are sorted + deduplicated for stable prompt hash.
        config.known_scenes.sort();
        config.known_scenes.dedup();

        Ok(Self {
            model: Arc::new(model),
            backend,
            model_path: path.to_path_buf(),
            config,
        })
    }

    /// Replace known scene list at runtime.
    pub fn set_known_scenes(&mut self, scenes: Vec<String>) {
        let mut sorted = scenes;
        sorted.sort();
        sorted.dedup();
        self.config.known_scenes = sorted;
    }

    fn build_user_prompt(&self, case: &PendingCase) -> String {
        let features_block = case
            .features
            .iter()
            .map(|(k, v)| format!("  - {k}: {}", format_value(v)))
            .collect::<Vec<_>>()
            .join("\n");
        let scenes_block = if self.config.known_scenes.is_empty() {
            "(none registered)".into()
        } else {
            self.config.known_scenes.join(", ")
        };
        format!(
            "Known scenes: {scenes_block}\n\
             Features:\n{features_block}\n\
             Escalation reason: {}\n\
             Respond with ONE line of JSON.",
            case.reason
        )
    }

    fn run_inference(&self, prompt: &str) -> Result<String, ReflectError> {
        let ctx_params =
            LlamaContextParams::default().with_n_ctx(NonZeroU32::new(self.config.n_ctx));
        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| ReflectError::Backend(format!("context init: {e}")))?;

        let tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| ReflectError::Backend(format!("tokenize: {e}")))?;

        if tokens.len() as u32 >= self.config.n_ctx {
            return Err(ReflectError::Budget(format!(
                "prompt {} tokens ≥ n_ctx {}",
                tokens.len(),
                self.config.n_ctx
            )));
        }

        let batch_size = tokens.len().clamp(64, 512);
        let mut batch = LlamaBatch::new(batch_size, 1);
        let last_idx = tokens.len() as i32 - 1;
        for (i, tok) in tokens.iter().enumerate() {
            batch
                .add(*tok, i as i32, &[0], i as i32 == last_idx)
                .map_err(|e| ReflectError::Backend(format!("batch add: {e}")))?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| ReflectError::Backend(format!("decode prompt: {e}")))?;

        // Greedy sampling for determinism in v0.1 (no temperature noise).
        let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);

        let mut generated_tokens = Vec::<llama_cpp_2::token::LlamaToken>::new();
        let start_pos = tokens.len() as i32;
        for step in 0..self.config.max_new_tokens {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);
            if self.model.is_eog_token(token) {
                break;
            }
            generated_tokens.push(token);

            batch.clear();
            batch
                .add(token, start_pos + step as i32, &[0], true)
                .map_err(|e| ReflectError::Backend(format!("batch add gen: {e}")))?;
            ctx.decode(&mut batch)
                .map_err(|e| ReflectError::Backend(format!("decode gen: {e}")))?;
        }

        // token_to_str per-token — avoids the fixed-buffer bug in tokens_to_str
        // (which panicked on Qwen outputs > internal buffer size).
        let mut output = String::with_capacity(generated_tokens.len() * 4);
        for tok in &generated_tokens {
            let piece = self
                .model
                .token_to_str(*tok, Special::Tokenize)
                .map_err(|e| ReflectError::Backend(format!("detokenize: {e}")))?;
            output.push_str(&piece);
        }
        Ok(output)
    }

    fn reflect_sync(&self, case: &PendingCase) -> Result<Reflection, ReflectError> {
        // Build chat-style prompt using the model's own template if available.
        let user_msg = self.build_user_prompt(case);
        let prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            self.config.system_prompt, user_msg
        );

        let raw = self.run_inference(&prompt)?;
        tracing::debug!(target: "perceptkit::reflector::local", "raw output: {raw}");

        parse_reflection(&raw)
    }
}

/// Parse the first valid JSON object from free-form model output into a Reflection.
fn parse_reflection(raw: &str) -> Result<Reflection, ReflectError> {
    // Find first `{` and its matching `}`. Qwen-0.5B sometimes emits whitespace / preamble.
    let start = raw
        .find('{')
        .ok_or_else(|| ReflectError::InvalidProposal(format!("no JSON object in: {raw:?}")))?;
    let sub = &raw[start..];

    // Brute-force brace matching (adequate for small responses).
    let mut depth = 0;
    let mut end = None;
    for (i, c) in sub.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let json = &sub[..end
        .ok_or_else(|| ReflectError::InvalidProposal(format!("unterminated JSON in: {raw:?}")))?];

    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| ReflectError::InvalidProposal(format!("JSON parse: {e} in {json:?}")))?;

    let kind = value
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ReflectError::InvalidProposal(format!("no 'kind' field in {json:?}")))?;

    match kind {
        "map" => {
            let scene_id = value
                .get("scene_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ReflectError::InvalidProposal("missing scene_id".into()))?
                .to_string();
            let rationale = value
                .get("rationale")
                .and_then(|v| v.as_str())
                .unwrap_or("(no rationale)")
                .to_string();
            Ok(Reflection::Map {
                scene_id,
                rationale,
            })
        }
        "propose" => {
            let yaml = value
                .get("yaml")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ReflectError::InvalidProposal("missing yaml".into()))?
                .to_string();
            let reason = value
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("(no reason)")
                .to_string();
            Ok(Reflection::Propose {
                yaml,
                examples: vec![reason],
            })
        }
        "unknown" => {
            let summary = value
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("(no summary)")
                .to_string();
            let top_features = value
                .get("top_features")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Ok(Reflection::Unknown {
                summary,
                top_features,
            })
        }
        other => Err(ReflectError::InvalidProposal(format!(
            "unknown kind '{other}'"
        ))),
    }
}

fn format_value(v: &serde_yml::Value) -> String {
    match v {
        serde_yml::Value::Number(n) => n.to_string(),
        serde_yml::Value::Bool(b) => b.to_string(),
        serde_yml::Value::String(s) => format!("\"{s}\""),
        other => serde_yml::to_string(other).unwrap_or_default(),
    }
}

#[async_trait]
impl Reflector for LocalReflector {
    async fn reflect(&self, case: PendingCase) -> Result<Reflection, ReflectError> {
        // llama.cpp inference is sync; running inside async fn is fine for
        // low-concurrency use. For high-concurrency, wrap with tokio::task::spawn_blocking.
        self.reflect_sync(&case)
    }

    fn name(&self) -> &'static str {
        "local"
    }

    fn fingerprint(&self) -> PromptHash {
        // Fingerprint includes model path + known_scenes to invalidate traces
        // when either changes.
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.model_path.hash(&mut h);
        self.config.known_scenes.hash(&mut h);
        self.config.system_prompt.hash(&mut h);
        PromptHash(format!("local@{:x}", h.finish()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_map_reflection() {
        let raw = r#"{"kind":"map","scene_id":"office_quiet","rationale":"silent"}"#;
        let r = parse_reflection(raw).unwrap();
        matches!(r, Reflection::Map { .. });
        if let Reflection::Map { scene_id, .. } = r {
            assert_eq!(scene_id, "office_quiet");
        }
    }

    #[test]
    fn parse_propose_reflection() {
        let raw =
            r#"preamble... {"kind":"propose","yaml":"id: new","reason":"novel features"} trailing"#;
        let r = parse_reflection(raw).unwrap();
        matches!(r, Reflection::Propose { .. });
    }

    #[test]
    fn parse_unknown_reflection() {
        let raw = r#"{"kind":"unknown","summary":"cannot classify","top_features":["a","b"]}"#;
        let r = parse_reflection(raw).unwrap();
        if let Reflection::Unknown {
            top_features,
            summary,
        } = r
        {
            assert_eq!(top_features.len(), 2);
            assert_eq!(summary, "cannot classify");
        } else {
            panic!("expected Unknown");
        }
    }

    #[test]
    fn parse_rejects_no_json() {
        assert!(parse_reflection("just text no json").is_err());
    }

    #[test]
    fn parse_rejects_unknown_kind() {
        assert!(parse_reflection(r#"{"kind":"xxx"}"#).is_err());
    }

    #[test]
    fn default_config_values() {
        let c = LocalConfig::default();
        assert_eq!(c.n_ctx, 2048);
        assert_eq!(c.max_new_tokens, 256);
    }
}
