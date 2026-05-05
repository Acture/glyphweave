# GlyphWeave

[![Crates.io](https://img.shields.io/crates/v/glyphweave)](https://crates.io/crates/glyphweave)
[![Release Build](https://github.com/Acture/glyphweave/actions/workflows/release.yml/badge.svg)](https://github.com/Acture/glyphweave/actions/workflows/release.yml)
[![License](https://img.shields.io/crates/l/glyphweave)](LICENSE)

## Shape-constrained SVG word clouds, built for speed.

GlyphWeave is a fast Rust CLI + library for generating bold SVG word clouds inside text and shape masks with multiple layout engines, reproducible runs, and palette control.

- Fast by default
- Visual by design
- CLI + library

## Example Gallery

Generated with fixed seeds, `fast-grid`, and `fonts/Roboto-Regular.ttf`.

| RUST (`auto`) | AI (`complementary`) |
|---|---|
| ![RUST auto palette](docs/examples/example-fast-grid.svg) | ![AI complementary palette](docs/examples/example-complementary.svg) |

| DATA (`analogous`) | CODE (`vibrant`) |
|---|---|
| ![DATA analogous palette](docs/examples/example-analogous.svg) | ![CODE vibrant palette](docs/examples/example-vibrant.svg) |

Reproduce these assets:

```bash
bash docs/examples/generate.sh
```

## Why It Feels Different

- Fast layouts out of the box with `fast-grid`, plus `mcts`, `simulated-annealing`, `spiral-greedy`, and `random-baseline`
- Strong visual control with palette strategies, weighted words, rotations, and SVG output
- Reproducible runs through `--seed`, config files, and library integration for automation

## Install

```bash
cargo install glyphweave
```

Optional: include embedded Noto Sans SC at build time.

```bash
cargo install glyphweave --features embedded_fonts
```

## Quick Start

```bash
glyphweave \
  --text "RUST" \
  --words "cloud,speed,layout,mask,svg,grid" \
  --canvas-size 1400,800 \
  --algorithm fast-grid \
  --palette auto \
  --seed 42 \
  --output output.svg
```

Use weighted input from file:

```text
# words.txt
rust,3
cloud,2
layout,2
mask
svg
```

```bash
glyphweave --text "AI" --word-file words.txt --algorithm spiral-greedy --rotations 0,90 --output ai.svg
```

Multi-line shape text (avoids needing to escape newlines in the shell):

```bash
glyphweave --text-lines "DATA,SCIENCE" --word-file words.txt --output data-science.svg
```

`--text-lines` accepts comma-separated lines and joins them with `\n` internally;
each line is rendered centered, stacked vertically with a 20% line gap.

Show all flags:

```bash
glyphweave --help
```

### Image-shaped masks

Use any PNG with an alpha channel as the shape (alpha > threshold = inside):

```bash
glyphweave --shape-image my_logo.png --shape-image-threshold 127 \
  --words "rust,svg,layout,cloud" \
  --canvas-size 1200,800 --output cloud.svg
```

`--shape-image` is mutually exclusive with `--text`.

## Use Cases

- Design assets and posters with text-shaped SVG output that stays easy to post-process
- Data storytelling visuals where the shape matters as much as the words
- Scripted and batch generation pipelines through the Rust API or CLI configs

## Library Example

```rust
use glyphweave::{
	generate, load_font_from_file, AlgorithmKind, CanvasConfig, CloudRequest, FontSizeSpec,
	RenderOptions, ShapeConfig, StyleConfig, WordEntry,
};
use std::{path::Path, sync::Arc};

let font = load_font_from_file(Path::new("fonts/NotoSansSC-Regular.ttf"))?;
let result = generate(CloudRequest {
	canvas: CanvasConfig { width: 1200, height: 700, margin: 12 },
	shape: ShapeConfig { text: "DATA".into(), font_size: FontSizeSpec::AutoFit },
	words: vec![WordEntry::new("rust", 2.0), WordEntry::new("svg", 1.0)],
	style: StyleConfig::default(),
	algorithm: AlgorithmKind::FastGrid,
	ratio_threshold: 0.85,
	max_try_count: 10_000,
	seed: Some(7),
	font: Arc::new(font),
	render: RenderOptions::default(),
})?;
std::fs::write("cloud.svg", result.svg)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Algorithm Cheat Sheet

| Algorithm | Speed | Fill Quality | Best Use Case |
|---|---:|---:|---|
| `fast-grid` | High | High | Default production choice |
| `mcts` | Medium-Low | High | Search-driven quality improvements |
| `simulated-annealing` | Medium-Low | Medium-High | Stochastic optimization and exploration |
| `spiral-greedy` | Medium | Medium-High | Center-focused, stable visual structure |
| `random-baseline` | Low | Medium | Baseline and regression comparison |

## Fonts

- Default behavior: try system fonts automatically
- Use `--font <path>` to pin a `.ttf/.otf`
- Use `--choose-system-font` for interactive font selection
- Embedded font feature: `embedded_fonts` (off by default)
- Embedded font: Noto Sans SC, SIL Open Font License 1.1
- License text: `fonts/OFL-NotoSansSC.txt`

## Config

Config precedence (later overrides earlier):

1. `~/.config/glyphweave/config.toml` (or `$XDG_CONFIG_HOME/glyphweave/config.toml`)
2. `.glyphweave.toml` in current directory
3. `--config <path>`
4. CLI flags

Minimal example:

```toml
canvas_size = [1600, 900]
algorithm = "fast-grid"
palette = "analogous"
palette_base = "#0EA5E9"
ratio = 0.85
max_tries = 12000
rotations = [0, 90]
```

## Documentation

- [Architecture](docs/architecture.md)
- [Library API](docs/library-api.md)
- [Algorithms](docs/algorithms.md)
- [Tuning](docs/tuning.md)
- [Migration v0.2](docs/migration-v0.2.md)

## For Maintainers

The `Release` workflow supports tag-driven and manual publishing to GitHub Releases, crates.io, and the `Acture/homebrew-ac` tap.

- Secrets: `HOMEBREW_TAP_TOKEN`
- Manual inputs: `tag`, `upload_assets`, `publish_cargo`, `update_homebrew`
- crates.io publishing uses Trusted Publishing via GitHub OIDC; configure `Acture/glyphweave`, workflow `release.yml`, and environment `release` as a trusted publisher on crates.io
- The first publish of a brand-new crate can be done locally; once the crate exists, rerun the workflow or use future tags for Trusted Publishing
- Use the `release` environment if you gate publishing with environment approvals

## License

AGPL-3.0. See [LICENSE](LICENSE).
