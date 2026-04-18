//! Feature registry — typed lookup + Levenshtein typo detection.
//!
//! Providers (like `perceptkit-audio`) register their FeatureDescriptors here.
//! When SceneEngine loads YAML scenes, any feature reference is checked
//! against this registry; unknown keys → `Error::UnknownFeature` with
//! `did_you_mean` Levenshtein suggestion.

use std::collections::HashMap;

use crate::error::Error;
use crate::feature::{FeatureDescriptor, FeatureKey};

/// Registry of known feature descriptors.
///
/// Providers register their schema once; SceneEngine resolves YAML references
/// against this registry at load time.
#[derive(Debug, Default, Clone)]
pub struct FeatureRegistry {
    by_key: HashMap<FeatureKey, FeatureDescriptor>,
}

impl FeatureRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a descriptor. Replaces any previous one with the same key.
    pub fn register(&mut self, descriptor: FeatureDescriptor) -> Option<FeatureDescriptor> {
        self.by_key.insert(descriptor.key.clone(), descriptor)
    }

    /// Look up a descriptor by key.
    pub fn get(&self, key: &FeatureKey) -> Option<&FeatureDescriptor> {
        self.by_key.get(key)
    }

    /// All registered descriptors.
    pub fn iter(&self) -> impl Iterator<Item = (&FeatureKey, &FeatureDescriptor)> {
        self.by_key.iter()
    }

    /// Number of registered descriptors.
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// Find the closest known key by Levenshtein distance, for
    /// `did_you_mean?` error messages. Returns `None` if distance > 3
    /// (too different) or if registry is empty.
    pub fn closest_key(&self, query: &str) -> Option<String> {
        const MAX_DISTANCE: usize = 3;
        self.by_key
            .keys()
            .map(|k| {
                let dist = strsim::levenshtein(query, k.as_str());
                (dist, k.as_str())
            })
            .filter(|(d, _)| *d <= MAX_DISTANCE)
            .min_by_key(|(d, _)| *d)
            .map(|(_, k)| k.to_string())
    }

    /// Resolve a key, returning `Error::UnknownFeature` with `did_you_mean`
    /// if the key is unknown. Used by YAML loader.
    pub fn resolve_or_error(
        &self,
        key: &FeatureKey,
        scene_id: &str,
    ) -> Result<&FeatureDescriptor, Error> {
        self.get(key).ok_or_else(|| Error::UnknownFeature {
            key: key.as_str().to_string(),
            scene_id: scene_id.to_string(),
            did_you_mean: self.closest_key(key.as_str()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::{FeatureKind, TimeWindow};

    fn sample_descriptor(key: &str) -> FeatureDescriptor {
        FeatureDescriptor {
            key: FeatureKey::new(key).unwrap(),
            kind: FeatureKind::F64 {
                min: Some(0.0),
                max: Some(1.0),
            },
            unit: Some("ratio_0_1".into()),
            window: TimeWindow::Instant,
            source: "test@0.1.0".into(),
            version: 1,
        }
    }

    #[test]
    fn register_and_get() {
        let mut reg = FeatureRegistry::new();
        reg.register(sample_descriptor("audio.voice_ratio"));
        let k = FeatureKey::new("audio.voice_ratio").unwrap();
        assert!(reg.get(&k).is_some());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn closest_key_finds_typo() {
        let mut reg = FeatureRegistry::new();
        reg.register(sample_descriptor("audio.voice_ratio"));
        reg.register(sample_descriptor("audio.speaker_count"));

        // Classic typo from plan.md §2.6 DoD
        assert_eq!(
            reg.closest_key("audio.voice_ratios"),
            Some("audio.voice_ratio".to_string())
        );
        // Close but also ambiguous → picks smaller distance one
        assert_eq!(
            reg.closest_key("audio.voice_ratio_x"),
            Some("audio.voice_ratio".to_string())
        );
    }

    #[test]
    fn closest_key_returns_none_for_very_different() {
        let mut reg = FeatureRegistry::new();
        reg.register(sample_descriptor("audio.voice_ratio"));
        assert_eq!(reg.closest_key("totally.unrelated.key_name"), None);
    }

    #[test]
    fn resolve_or_error_gives_did_you_mean() {
        let mut reg = FeatureRegistry::new();
        reg.register(sample_descriptor("audio.voice_ratio"));
        let k = FeatureKey::new("audio.voice_ratios").unwrap();
        let err = reg.resolve_or_error(&k, "online_meeting").unwrap_err();
        match err {
            Error::UnknownFeature {
                key,
                scene_id,
                did_you_mean,
            } => {
                assert_eq!(key, "audio.voice_ratios");
                assert_eq!(scene_id, "online_meeting");
                assert_eq!(did_you_mean, Some("audio.voice_ratio".to_string()));
            }
            _ => panic!("wrong error variant"),
        }
    }
}
