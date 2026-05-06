# Library API

## Public Types

- `CloudRequest`: input configuration
- `CloudResult`: output SVG + placements + stats
- `AlgorithmKind`: `FastGrid` / `SpiralGreedy` / `RandomBaseline` / `Mcts` / `SimulatedAnnealing`
- `CanvasConfig`, `ShapeConfig`, `StyleConfig`, `WordEntry`, `RenderOptions`
- `ShapeSource`: enum with `Text { text, font_size }` and `Image { path, threshold }` variants. Construct via `ShapeConfig::text(...)` or `ShapeConfig::image(...)` helpers.
- `FontSizeSpec`: enum with `Fixed(usize)` and `AutoFit` variants. Used by `ShapeSource::Text`; `AutoFit` triggers binary-search sizing in `mask::calculate_auto_font_size`.
- `CloudPlacement`: per-word placement record (text, color, position, font size, rotation) emitted in `CloudResult::placements` and used by `render::render_svg`.
- `Rotation`: newtype `pub struct Rotation(pub u16)`. Accepts any integer degrees in `0..=360`; previously restricted to `0` / `90`. Constants `Rotation::Deg0` / `Rotation::Deg90` remain available.
- `CloudStats`: includes `elapsed: Duration` (replaces the old `elapsed_ms: u128`) and `internal_evaluations: usize` for the true number of placement-attempt evaluations performed by the chosen algorithm. `shape_font_size` is `Option<usize>` — `None` for image-mask shapes.
- `RenderMetadata`: bundle handed to `render::render_svg` carrying seed, algorithm, and stats; embedded into the SVG output as a `<metadata>` element.
- `GlyphWeaveError`: the `Io` variant now carries `{ path, source }` for richer error context.

## Entry Point

```rust
pub fn generate(request: CloudRequest) -> Result<CloudResult, GlyphWeaveError>
```

## Font Loading Helpers

```rust
pub fn load_font_from_file<P: AsRef<Path>>(path: P) -> Result<Font, GlyphWeaveError>
pub fn load_default_embedded_font() -> Result<Font, GlyphWeaveError>
pub fn discover_system_font_candidates() -> Vec<PathBuf>
pub fn load_system_font() -> Result<(Font, PathBuf), GlyphWeaveError>
```

`load_default_embedded_font()` requires `embedded_fonts` feature.

## Minimal Example

```rust
use glyphweave::*;
use std::sync::Arc;

let font = load_font_from_file("fonts/NotoSansSC-Regular.ttf")?;
let request = CloudRequest {
    canvas: CanvasConfig::default(),
    shape: ShapeConfig::text("HELLO", FontSizeSpec::AutoFit),
    words: vec![WordEntry::new("rust", 2.0), WordEntry::new("svg", 1.0)],
    style: StyleConfig::default(),
    algorithm: AlgorithmKind::FastGrid,
    ratio_threshold: 0.85,
    max_try_count: 10_000,
    seed: Some(42),
    font: Arc::new(font),
    render: RenderOptions::default(),
};

let result = generate(request)?;
std::fs::write("output.svg", result.svg)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Build with `--features embedded_fonts` if you want to call `load_default_embedded_font()`.

## Determinism

Set `seed` to a fixed value to make layout output reproducible for snapshots and regression tests.

## Serde Support

`CloudResult`, `CloudPlacement`, `CloudStats`, and `Rotation` implement
`serde::Serialize`. Use any serde-compatible serializer (e.g. `serde_json`)
to export placements for downstream consumption (D3, web frontends, etc.).

`Rotation` serializes as a numeric value (any integer in `0..=360`) rather
than a string. `CloudStats.elapsed` serializes as a `Duration` (seconds +
nanoseconds); call `.as_millis()` if a single number is preferred for
display.

## Metadata in Output

Every generated SVG now embeds a `<metadata>` element with the seed used,
algorithm name, internal evaluation count, and elapsed time. This makes
generated assets self-describing for reproducibility: a single SVG file
contains enough context to regenerate it (`--seed`, algorithm, stats).
The metadata is also written to stderr (the seed in particular is always
echoed to stderr so scripted runs can capture it even when a random seed
is auto-selected).

## Strict TOML Configuration

Config files are parsed with `deny_unknown_fields`: typos or unsupported
keys produce an error rather than being silently ignored. This catches
accidental drift between docs and config schema.
