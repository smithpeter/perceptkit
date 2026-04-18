# Changelog

All notable changes to perceptkit will be documented in this file.

Follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **M1 Scaffold (2026-04-18)**
  - Cargo workspace with 3 crates: `perceptkit-core` / `perceptkit-audio` / `perceptkit-py`
  - PyO3 + maturin build configuration (`abi3-py311`)
  - Dual licensing (MIT OR Apache-2.0)
  - `cargo deny` configuration enforcing Signal Model (no network crates)
  - GitHub Actions CI (Rust fmt / clippy / test + cargo-deny + DCO check)
  - STRATEGY.md (11 sections, North Star doc)
  - plan.md (66-day v0.1 roadmap with D45/D55/D60 kill-switches)
  - DATA.md (Signal Model, Datasheet, Model Card, Eval Card, DCO protocol)
  - NAMING.md (SceneKit → Percept → perceptkit history)
  - CONTRIBUTING.md with DCO protocol

### Governance
- Three rounds of red/blue team adversarial review (R1-R3, weighted 7.25/10 GO)
- Data strategy Round D1 with 4-perspective review (architecture+privacy / product+community / business+investment / QA+datascience)
