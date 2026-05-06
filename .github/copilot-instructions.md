# GlyphWeave

Rust CLI + library that generates shape-constrained SVG word clouds. Crate name `glyphweave` (Rust edition 2024, AGPL-3.0).

## Build, test, lint

CI runs (must pass before merge — see `.github/workflows/ci.yml`):

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings   # warnings are errors
cargo test --release --lib --bins --tests
cargo test --release --features embedded_fonts --lib --bins --tests
```

Use `--release`: debug-mode SpiralGreedy walks ~10⁶ offsets per attempt and a single `cargo test` run on all five regression snapshots can take 5+ minutes. Release brings it to seconds.

- Run a single test: `cargo test --test regression_snapshots layout_svg_snapshots_are_stable` (or `cargo test <name>` for `#[test]` fns inside the crate).
- Run the CLI from source: `cargo run --bin glyphweave -- --text RUST --words rust,svg --font fonts/NotoSansSC-Regular.ttf --output out.svg --no-progress`.
- Build with bundled font: `cargo build --features embedded_fonts`. Without this feature, `load_default_embedded_font()` returns an error and tests/CLI fall back to `fonts/NotoSansSC-Regular.ttf` (a Git LFS asset — `git lfs pull` if missing).
- Benches: `cargo bench --bench layout_bench` (Criterion, `harness = false`).
- Perf-regression tests are `#[ignore]`d; run them with `cargo test --tests -- --ignored` (the `perf.yml` workflow does this on a schedule).

## Architecture

The crate is intentionally split into a reusable library and a thin CLI shell — keep that boundary intact.

- `src/lib.rs` — public library surface. The single entry point is `generate(CloudRequest) -> Result<CloudResult, GlyphWeaveError>`. It validates → resolves font size (`FontSizeSpec::AutoFit` or `Fixed`) → rasterizes the shape into a boolean mask (`src/mask.rs`) → dispatches to a `LayoutStrategy` → renders SVG (`src/render.rs`) → returns placements + `CloudStats`.
- `src/core/{model,error}.rs` — shared types. All public types are re-exported from `lib.rs`; add new public types there too.
- `src/layout/` — pluggable algorithms behind the `LayoutStrategy` trait (`mod.rs`). Each algorithm is its own module (`fast_grid`, `spiral_greedy`, `random_baseline`, `mcts`, `simulated_annealing`). Shared helpers (rect math, weighted word picking, candidate sampling, progress bar, integral-image utilities) live in `src/layout/common.rs` — reuse them rather than duplicating.
- `src/bin/glyphweave.rs` — CLI entry. It pulls in the CLI-only modules with `#[path = "../cli/mod.rs"] mod cli;` because `src/cli/` is **not** exposed through the library. Don't add `pub mod cli` to `lib.rs`.
- `src/cli/{args,config,palette}.rs` — clap parser, layered TOML config, palette resolution. Config precedence (later overrides earlier, non-`None` wins): `$XDG_CONFIG_HOME/glyphweave/config.toml` → `./.glyphweave.toml` → `--config <path>` → CLI flags. `FileConfig::merge_from` enforces this — mirror the pattern when adding new options.
- `src/embedded_fonts.rs` — gated by the `embedded_fonts` feature; ships Noto Sans SC. Library code never assumes it's available.

### Adding a new layout algorithm

1. Add a module under `src/layout/`, implement `LayoutStrategy::place`.
2. Re-export the strategy from `src/layout/mod.rs` and add a variant to `AlgorithmKind` (`src/core/model.rs`).
3. Wire the variant into the inline `match request.algorithm` in `src/lib.rs::generate` (W7 P10 removed the older `strategy_for` indirection).
4. Add a `CliAlgorithm` variant + `parse_text` aliases in `src/cli/args.rs` and the `From<CliAlgorithm> for AlgorithmKind` arm.
5. Add a snapshot case to `tests/regression_snapshots.rs` and regenerate goldens (see below).

### Error model and exit codes

`GlyphWeaveError` variants map to fixed CLI exit codes in `bin/glyphweave.rs::map_error_to_exit_code`: `InvalidConfig=2`, `FontLoad=3`, `Io|Image=4`, `Generation=5`. Tests assert on these (e.g. `cli_returns_invalid_config_exit_code_when_words_missing`) — pick the right variant when adding new failure paths.

## Conventions

- **Formatting:** `rustfmt.toml` sets `hard_tabs = true`; `.editorconfig` enforces tabs and a 200-char line limit. `cargo fmt --check` is gating.
- **Determinism:** `seed: Option<u64>` on `CloudRequest` must produce byte-identical SVG output. Snapshot tests (`tests/regression_snapshots.rs`) and `tests/api_validation.rs::generate_with_same_seed_is_stable` enforce this. Don't introduce nondeterminism (e.g. `HashMap` iteration over placements, unseeded RNG) into the layout pipeline.
- **Snapshots:** Goldens live in `tests/golden/<algorithm>.svg.snap`. After an intentional layout/render change, regenerate with `UPDATE_GOLDEN=1 cargo test --test regression_snapshots` and commit the updated snapshots. SVG is whitespace-normalized before comparison (see `tests/support/mod.rs::normalize_svg`).
- **Test fixtures:** Integration tests build requests via `tests/support/mod.rs::build_request`, which loads the embedded font when available and falls back to `fonts/NotoSansSC-Regular.ttf`. Reuse it instead of hand-rolling `CloudRequest`s.
- **Validation:** `CloudRequest::validate` is the single source of truth for request-level invariants. Add new constraints there (returning `GlyphWeaveError::InvalidConfig`) rather than scattering checks across algorithms.
- **Commits:** `cliff.toml` requires Conventional Commits — `feat:`, `fix:`, `doc:`, `perf:`, `refactor:`, `style:`, `test:`, `chore:`, `ci:`, `revert:`. Non-conforming commits are filtered out of the changelog.
- **Git LFS:** `.gitattributes` tracks `*.ttf` via LFS. Workflows check out with `lfs: true`; do the same in any new workflow.
- **Generated artifacts:** `*.svg`, `*.png`, `/result`, `/artifacts/` are gitignored (sample outputs in `docs/examples/` are the documented exception, regenerated via `bash docs/examples/generate.sh`).

## Documentation to keep in sync

User-facing changes usually require touching matching docs:
- New CLI flag → `README.md` quick-start + relevant section under `docs/`.
- New algorithm → `docs/algorithms.md` and the "Algorithm Cheat Sheet" table in `README.md`.
- Public API change → `docs/library-api.md` and the `pub use` block in `src/lib.rs`.
- Tuning guidance → `docs/tuning.md`.

## Release

Tags `v*` trigger `.github/workflows/release.yml` (re-runs fmt/clippy/tests, then publishes to GitHub Releases, crates.io via OIDC Trusted Publishing, and the `Acture/homebrew-ac` tap). Bump `Cargo.toml` `version`, prepend a `## [x.y.z]` section to `CHANGELOG.md`, then tag.
