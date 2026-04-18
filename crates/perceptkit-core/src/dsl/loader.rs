//! Scene YAML loader — reads a directory of `.yaml` files into `Vec<Scene>`.
//!
//! Validates each scene's feature references against a `FeatureRegistry`
//! (optional — if registry is empty, skips validation). Emits
//! `Error::UnknownFeature` with Levenshtein `did_you_mean` suggestion.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::feature::FeatureKey;
use crate::registry::FeatureRegistry;
use crate::scene::Scene;

/// Load a single scene file.
pub fn load_file(path: &Path) -> Result<Scene> {
    let content = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: Some(path.to_path_buf()),
        source,
    })?;
    serde_yml::from_str::<Scene>(&content).map_err(|e| Error::YamlParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

/// Load every `.yaml` / `.yml` file under `dir` (non-recursive).
///
/// Returns scenes sorted by descending priority (ties broken by id).
/// Validates feature references against `registry` (skip when empty).
pub fn load_dir(dir: &Path, registry: &FeatureRegistry) -> Result<Vec<Scene>> {
    let read = std::fs::read_dir(dir).map_err(|source| Error::Io {
        path: Some(dir.to_path_buf()),
        source,
    })?;

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in read {
        let entry = entry.map_err(|source| Error::Io {
            path: Some(dir.to_path_buf()),
            source,
        })?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let is_yaml = p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"));
        if is_yaml {
            paths.push(p);
        }
    }
    // Deterministic ordering — file name asc, then priority desc handled below.
    paths.sort();

    let mut scenes = Vec::with_capacity(paths.len());
    let mut seen_ids: HashSet<String> = HashSet::new();
    for p in &paths {
        let scene = load_file(p)?;
        if !seen_ids.insert(scene.id.clone()) {
            return Err(Error::DuplicateScene(scene.id));
        }
        if !registry.is_empty() {
            validate_features(&scene, registry)?;
        }
        scenes.push(scene);
    }

    // Priority desc, then id asc for stable tie-break.
    scenes.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));

    Ok(scenes)
}

/// Validate that every `feature:` reference in a scene resolves in the registry.
/// Emits `Error::UnknownFeature { did_you_mean }` for typos.
pub fn validate_features(scene: &Scene, registry: &FeatureRegistry) -> Result<()> {
    for cond in scene.match_rules.all_conditions() {
        let key = FeatureKey::new(&cond.feature).map_err(|_| Error::InvalidScene {
            scene_id: scene.id.clone(),
            message: format!("invalid feature key syntax: '{}'", cond.feature),
        })?;
        registry.resolve_or_error(&key, &scene.id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::{FeatureDescriptor, FeatureKind, TimeWindow};

    fn descriptor(k: &str) -> FeatureDescriptor {
        FeatureDescriptor {
            key: FeatureKey::new(k).unwrap(),
            kind: FeatureKind::F64 {
                min: Some(0.0),
                max: Some(1.0),
            },
            unit: None,
            window: TimeWindow::Instant,
            source: "test".into(),
            version: 1,
        }
    }

    fn write_scene(dir: &Path, name: &str, yaml: &str) {
        std::fs::write(dir.join(name), yaml).unwrap();
    }

    #[test]
    fn load_file_parses_scene() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("s.yaml");
        std::fs::write(
            &p,
            r#"
id: s
version: 1
describe:
  template: x
match:
  all: []
"#,
        )
        .unwrap();
        let s = load_file(&p).unwrap();
        assert_eq!(s.id, "s");
    }

    #[test]
    fn load_dir_sorts_by_priority() {
        let tmp = tempfile::tempdir().unwrap();
        write_scene(
            tmp.path(),
            "a.yaml",
            "id: a\nversion: 1\ndescribe: {template: a}\nmatch: {}\npriority: 5\n",
        );
        write_scene(
            tmp.path(),
            "b.yaml",
            "id: b\nversion: 1\ndescribe: {template: b}\nmatch: {}\npriority: 20\n",
        );
        let scenes = load_dir(tmp.path(), &FeatureRegistry::new()).unwrap();
        assert_eq!(scenes[0].id, "b");
        assert_eq!(scenes[1].id, "a");
    }

    #[test]
    fn duplicate_scene_id_errors() {
        let tmp = tempfile::tempdir().unwrap();
        write_scene(
            tmp.path(),
            "1.yaml",
            "id: dup\nversion: 1\ndescribe: {template: x}\nmatch: {}\n",
        );
        write_scene(
            tmp.path(),
            "2.yaml",
            "id: dup\nversion: 1\ndescribe: {template: y}\nmatch: {}\n",
        );
        let err = load_dir(tmp.path(), &FeatureRegistry::new()).unwrap_err();
        matches!(err, Error::DuplicateScene(_));
    }

    #[test]
    fn unknown_feature_gives_did_you_mean() {
        let tmp = tempfile::tempdir().unwrap();
        write_scene(
            tmp.path(),
            "s.yaml",
            r#"
id: meeting
version: 1
describe:
  template: meeting
match:
  all:
    - { feature: audio.voice_ratios, op: gt, value: 0.4 }
"#,
        );
        let mut reg = FeatureRegistry::new();
        reg.register(descriptor("audio.voice_ratio"));
        let err = load_dir(tmp.path(), &reg).unwrap_err();
        if let Error::UnknownFeature {
            key, did_you_mean, ..
        } = err
        {
            assert_eq!(key, "audio.voice_ratios");
            assert_eq!(did_you_mean, Some("audio.voice_ratio".to_string()));
        } else {
            panic!("expected UnknownFeature");
        }
    }
}
