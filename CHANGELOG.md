## [0.5.0] - 2026-05-06

### 🚀 Features

- *(api)* Derive serde::Serialize for output types
- *(cli)* Always echo used seed and key stats to stderr
- *(config)* [**breaking**] Reject unknown TOML fields with deny_unknown_fields
- *(error)* [**breaking**] Include path context in IO and image errors
- *(cli)* Expose --word-padding for inter-word spacing
- *(stats)* Unify attempts semantics across strategies, add internal_evaluations
- *(shape)* Support multi-line shape text
- *(api)* [**breaking**] Support image masks via ShapeSource enum
- *(api)* [**breaking**] Extend Rotation to support arbitrary 0..360 degrees
- *(stats)* [**breaking**] Change CloudStats.elapsed_ms to Duration
- *(cli)* Default --word-padding to 2 for readable output
- *(render)* Embed seed and stats in SVG metadata for self-describing output
- *(shape)* Use fontdue line metrics for multi-line line-height
- *(api)* [**breaking**] Re-export GlyphWeaveError, non_exhaustive enums + output structs, drop ZERO/NINETY
- *(api)* [**breaking**] Privatize Rotation field; new() returns Result rejecting >= 360
- *(render)* Add SVG <title> + role=img for a11y; thread TextSizeCache

### 🐛 Bug Fixes

- *(render)* Correct 90° rotation pivot to align with reserved mask rect
- *(simulated-annealing)* Correct acceptance criterion semantics
- *(rng)* Replace modulo bias with rejection sampling in random_index
- *(render)* Use text-before-edge baseline for pixel-accurate placement
- *(test)* Drop flaky timing assertion from image_mask large-input test
- *(layout)* Clear pending_rects entirely in IncrementalAvailability::restore
- *(cli)* Make --shape-image / --text / --text-lines mutually exclusive

### 💼 Other

- Extend layout_bench to cover MCTS and SimulatedAnnealing

### 🚜 Refactor

- *(error)* Generalize Io and Image error wording
- *(layout)* [**breaking**] Move BitMask to core to break mask↔layout circularity

### 📚 Documentation

- *(spiral-greedy)* Clarify that this is a rectangular, not Archimedean, spiral
- Update README and docs for v0.4.0 API surface
- Add CONTRIBUTING.md with workflow and conventions
- Add SECURITY.md policy
- Add GitHub issue templates
- Add .github/copilot-instructions.md to repo + sync stale references
- README spiral-greedy example & tuning budget for post-Archimedean
- *(migration)* Add v0.4 migration guide for the 5 breaking changes

### ⚡ Performance

- *(layout)* Share incremental integral image across all strategies
- *(spiral-greedy)* Cache spiral offsets, fix capacity, recenter on geometric mean
- *(layout)* Cache calculate_text_size per generate()
- *(mask)* Replace Array2<bool> with bit-packed BitMask
- *(mcts)* Rollout uses diff-and-rollback instead of mask clone
- *(layout)* Inline strategy dispatch, drop Box<dyn LayoutStrategy>
- *(spiral)* Prune offsets by canvas bounds and per-canvas cache
- *(mask)* Use Nearest filter for image_mask resize
- *(spiral)* Replace rectangular spiral with true Archimedean curve
- *(layout)* Zero-alloc TextSizeCache hit path
- *(layout)* Refill positions only on failure; spiral cache+doc fixes

### 🧪 Testing

- Add hand-rolled fuzz harness for user-input parsers
- Add property-based layout invariant tests via proptest
- Parametrize regression snapshots across 3 seeds per algorithm
- *(proptest)* Include MCTS and SimulatedAnnealing in property-based tests
- Strengthen proptest, fix rotated-rect assumption, add render+error tests

### ⚙️ Miscellaneous Tasks

- Switch crates.io publishing to trusted publishing
- Also run cargo test with --features embedded_fonts
- *(changelog)* Regenerate via git-cliff for 0.4.0
- *(release)* Prepare v0.4.0
- *(fuzz)* Add cargo-fuzz target for image_mask
- Add coverage job using cargo-llvm-cov + codecov
- Add bench-on-PR comparison workflow
- Bench-pr label gating, dedupe ci.yml test pass, fuzz cleanups
- [**breaking**] Rename SHAPECLOUD_FONT → GLYPHWEAVE_FONT; threshold in FileConfig; drop dead interactive arg
- [**breaking**] Layout common cleanup batch + config IO + CloudStats Option

## [0.3.0] - 2026-03-06

### 💥 Breaking Changes

- Rename the package, library crate, CLI binary, config paths, and public docs from `char-cloud` to `glyphweave`
- Standardize all public repository links, badges, and release metadata on `Acture/glyphweave`

### 🚀 Features

- Prepare the first fully branded `glyphweave` release for GitHub, crates.io, docs.rs, and Homebrew

### ⚙️ CI

- Remove legacy `char-cloud` Homebrew formula migration logic from the release workflow
- Publish future GitHub release assets and tap updates under the `glyphweave` name only

## [0.2.0] - 2026-03-06

### 🚀 Features

- Refactor project into a reusable library + CLI shell architecture
- Add pluggable layout strategies with `fast-grid`, `spiral-greedy`, `mcts`, and `simulated-annealing`
- Add weighted words, configurable rotations, and deterministic generation via seed
- Add palette strategies (`auto`, `complementary`, `triadic`, `analogous`, `monochrome`, presets)
- Add feature-gated embedded font support with system font fallback

### 🐛 Bug Fixes

- Correct release workflow tag trigger pattern for v-prefixed tags
- Improve release workflow compatibility with Git LFS font assets

### 📚 Documentation

- Rewrite README in concise English style and add generated SVG gallery examples
- Add architecture, API, algorithms, tuning, and migration docs for v0.2
- Add embedded font license documentation for Noto Sans SC

### 🧪 Testing

- Add snapshot regression tests, config precedence tests, and performance regression checks
- Expand API/CLI integration coverage for new configuration and algorithm options

### ⚙️ CI

- Add dedicated CI and performance regression workflows
- Harden release pipeline checks (`fmt`, `clippy`, `tests`) before publishing assets

## [0.1.2-test] - 2025-08-08

### 🚀 Features

- *(ci)* Add support for Git ref input in release workflow

### 🐛 Bug Fixes

- *(ci)* Correct draft flag logic in release workflow
- *(ci)* Update default Git ref in release workflow

### 🚜 Refactor

- *(ci)* Simplify and streamline release workflow
- *(ci)* Remove manual dispatch inputs from release workflow

### 📚 Documentation

- *(readme)* Update README with English translation and improved formatting
## [0.1.2] - 2025-06-11

### 🚀 Features

- *(draw)* Add progress bar for text filling process
## [0.1.1] - 2025-06-11

### 🚀 Features

- *(flakes)* Add Nix Flake support for project development and builds
- *(ci)* Add GitHub Actions workflow for release builds
- *(main, embedded_fonts)* Add conditional support for embedded fonts
- *(ci)* Add manual trigger to release workflow
- *(ci, mask)* Enhance release workflow and fix module import
- *(flakes, ci)* Improve cross-platform builds and dev environment
- *(ci)* Enhance release workflow with improved platform support and artifact handling
- *(ci)* Refine release workflow with manual draft option and enhanced artifact handling
- *(ci)* Update release workflow with formal release option

### 🐛 Bug Fixes

- *(main)* Improve logging levels and clean up formatting
- *(utils, args, draw, mask, main)* Clean up formatting and optimize code consistency
- *(ci)* Update release workflow for Nix build command
- *(mask, ci)* Resolve module import and simplify workflow steps
- *(ci)* Remove unsupported aarch64-windows target and redundant steps
- *(ci)* Add `contents: write` permission for release workflow
- *(ci)* Simplify checksum generation in release workflow
- *(ci)* Remove redundant `make_latest` option in release workflow
- *(cargo)* Bump version to 0.1.1
- *(ci)* Enable Git LFS support in release workflow

### 📚 Documentation

- *(readme)* Update badge to display release build status

### ⚙️ Miscellaneous Tasks

- *(ci)* Update GitHub Actions to latest versions
## [1.0.0] - 2025-06-10

### 🚀 Features

- Initialize project with basic setup
- *(mask)* Add text rendering and mask generation utilities
- *(draw)* Add text drawing utilities and integrate with mask generation
- *(draw)* Improve text placement logic and introduce dynamic font sizing
- *(fonts)* Add NotoSansSC-Regular font file
- *(repo)* Add .gitattributes for LFS font file management
- *(draw)* Add support for configurable text colors
- *(config)* Add default values for new text and canvas configurations
- *(cli)* Add command-line interface for canvas and drawing configuration
- *(embedded_fonts)* Centralize font data management in a new module
- *(cli)* Enhance CLI with new options and improve canvas configuration

### 🐛 Bug Fixes

- *(draw)* Update font loading path and adjust editorconfig
- *(draw, mask)* Update font handling and improve configuration consistency

### 📚 Documentation

- *(readme)* Add project description, features, usage, and license details

### ⚙️ Miscellaneous Tasks

- *(gitignore)* Update gitignore for SVG and PNG assets
- *(gitignore)* Remove `python` from ignore list
- *(cargo)* Update project metadata in Cargo.toml
