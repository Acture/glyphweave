# Library API

## Public Types

- `CloudRequest`: input configuration
- `CloudResult`: output SVG + placements + stats
- `AlgorithmKind`: `FastGrid` / `SpiralGreedy` / `RandomBaseline`
- `CanvasConfig`, `ShapeConfig`, `StyleConfig`, `WordEntry`, `RenderOptions`

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
    shape: ShapeConfig { text: "HELLO".into(), font_size: FontSizeSpec::AutoFit },
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

`Rotation` serializes as a numeric value (`0` or `90`) rather than a string.
