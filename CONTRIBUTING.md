# Contributing to GlyphWeave

Thanks for considering a contribution! This document captures the
workflow, conventions, and required checks. The repository is licensed
AGPL-3.0; by contributing you agree your work is licensed the same.

## Workflow

1. Fork `Acture/glyphweave` and create a feature branch from `master`:
   ```bash
   git checkout -b feat/your-change master
   ```
2. Make focused, atomic commits using **Conventional Commits**
   (enforced by `cliff.toml` for changelog generation):
   - `feat:`, `fix:`, `perf:`, `refactor:`, `docs:`, `test:`, `chore:`,
     `ci:`, `style:`, `revert:`
   - Add `!` for breaking changes, e.g. `feat(api)!: rename FooConfig`
   - Scope optional: `feat(layout): ...`, `fix(render): ...`
3. Run the required checks (see below) and push to your fork.
4. Open a PR against `master`. Reference any related issue.

## Required checks

These must pass before review (CI runs the same):

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --bins --tests
cargo test --features embedded_fonts --lib --bins --tests
```

For algorithm or rendering changes, also regenerate snapshots if
intentional:

```bash
UPDATE_GOLDEN=1 cargo test --release --test regression_snapshots
```

## Project layout

- `src/lib.rs` — public library API + `generate()` orchestration
- `src/core/` — shared types (`CloudRequest`, `CloudResult`, errors)
- `src/layout/` — pluggable layout strategies behind `LayoutStrategy`
- `src/mask.rs` — shape rasterization to `BitMask`
- `src/render.rs` — SVG assembly
- `src/bin/glyphweave.rs` — CLI entrypoint (uses `#[path = "../cli/mod.rs"]`)
- `src/cli/` — clap parser, layered TOML config, palettes
- `tests/` — integration tests, regression snapshots, proptest, fuzz harness
- `benches/` — Criterion bench harness
- `fuzz/` — cargo-fuzz nightly targets (optional)

See `docs/architecture.md` for the data flow.

## Adding a new layout algorithm

1. Add a module under `src/layout/`.
2. Implement the `LayoutStrategy` trait.
3. Add a variant to `AlgorithmKind` in `src/core/model.rs`.
4. Wire the variant into the inline match in `src/lib.rs::generate`.
5. Add a `CliAlgorithm` variant + `parse_text` aliases in `src/cli/args.rs`.
6. Add a regression snapshot case in `tests/regression_snapshots.rs`
   and regenerate golden files.
7. Update `docs/algorithms.md`.

## Determinism

The library guarantees byte-identical SVG output for a fixed `seed`.
Don't introduce nondeterministic data structures (e.g. `HashMap`
iteration over placements) into the layout pipeline. The
`tests/regression_snapshots.rs` and proptest suites enforce this.

## Fuzz testing

- Stable Rust fallback: `cargo test --release --test fuzz_inputs`
- Real cargo-fuzz (nightly):
  ```bash
  rustup install nightly
  cargo install cargo-fuzz
  cargo +nightly fuzz run image_mask
  ```

## Releases

Maintainers tag `v0.x.y` from `master`; the `release.yml` workflow
publishes to GitHub Releases, crates.io (Trusted Publishing), and the
`Acture/homebrew-ac` tap. Bump `Cargo.toml`, regenerate `CHANGELOG.md`
via git-cliff, then tag.

## Code of conduct

Be kind. Assume good faith. We follow the
[Contributor Covenant](https://www.contributor-covenant.org/) v2.1.

## License

By contributing you agree your work is licensed under AGPL-3.0, the
same as the rest of the repository (see `LICENSE`).
