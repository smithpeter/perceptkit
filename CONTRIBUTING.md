# Contributing to perceptkit

Thanks for considering a contribution.

## Developer Certificate of Origin (DCO)

All commits **must** be signed off per the [Developer Certificate of Origin v1.1](https://developercertificate.org/).

We use DCO (not CLA). Our reason: Kubernetes / Linux / GitLab A/B data shows CLA reduces first-contributions by ~35% while DCO has minimal friction.

Use `git commit -s` to automatically add:

```
Signed-off-by: Your Name <your.email@example.com>
```

By signing off, you certify (abbreviated):

- (a) The contribution is your original work, and you have the right to submit it under the open source license indicated.
- (b) The contribution is based upon previous work under a compatible open source license.
- (c) You understand that all contributions are public and recorded indefinitely.

Full DCO text: <https://developercertificate.org/>

### Pre-commit Hook

To automatically sign-off all commits:

```bash
git config core.hooksPath .githooks
```

(We ship a minimal pre-commit hook that adds Signed-off-by if missing.)

## Code Style

- **Rust**: `cargo fmt` + `cargo clippy -- -D warnings` — both enforced by CI.
- **Python**: `ruff check` + `mypy --strict` — both enforced by CI.

## Signal Model

perceptkit-core **must not** introduce any network call or depend on any network crate. This is enforced by `cargo deny` in CI per [DATA.md §2](DATA.md#2-signal-model-承诺最硬核).

If you need a networked feature (e.g., an optional Cloud Reflector in v0.2+), it must be:
- A separate crate or opt-in feature flag
- Default-disabled
- Explicitly documented in DATA.md

## Data Contributions

Data contributions go to the separate `perceptkit-bench-v0` repository (to be created in M4).

All data must be **CC0** licensed and sign the same DCO.

See [DATA.md §6](DATA.md#6-contribution-protocol-dco) for the contribution workflow.

## Pull Request Process

1. Fork the repo and create a branch (`feature/your-feature` or `fix/issue-123`).
2. Make changes, `git commit -s` each commit.
3. Run local checks: `cargo fmt && cargo clippy -- -D warnings && cargo test`.
4. Open a PR against `main`.
5. Ensure all CI checks pass (fmt / clippy / test / cargo-deny / DCO).
6. Address review feedback.

## Governance

Strategic changes (to STRATEGY.md §1-§5, §11) require red/blue team adversarial review. See STRATEGY.md §9 for examples.

Execution-level changes (plan.md, code) can proceed through normal PR flow.

## Questions?

Open an issue or discussion. For private matters, email: zouyongming@gmail.com.
