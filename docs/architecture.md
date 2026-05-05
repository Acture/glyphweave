# Architecture

## Overview

GlyphWeave v0.2 splits the project into a reusable library core and a thin CLI wrapper.

- `src/lib.rs`: public API + generation orchestration
- `src/core/`: shared models and error types
- `src/mask.rs`: shape rasterization and mask utilities
- `src/layout/`: pluggable layout strategies
- `src/render.rs`: SVG assembly
- `src/bin/glyphweave.rs`: CLI entrypoint only

## Data Flow

1. Parse request (CLI or library caller).
2. Validate config (`CloudRequest::validate`).
3. Resolve `ShapeSource`:
   - `Text { text, font_size }`: rasterize text (auto-fit or fixed size) into boolean mask.
   - `Image { path, threshold }`: load PNG, resize to canvas, threshold alpha into boolean mask.
4. Run selected `LayoutStrategy` to place words.
5. Render placements to SVG string.
6. Return `CloudResult` with placements + stats.

## Layout Plugin Contract

`LayoutStrategy` consumes:

- immutable shape mask
- style/word/font configuration
- max tries + ratio threshold
- RNG source

It returns:

- placed words with coordinates/style
- attempts used
- occupied area count

This contract keeps algorithms isolated from CLI and rendering concerns.

## Error Model

All library operations return `Result<_, GlyphWeaveError>`.

- `InvalidConfig`: request-level validation failures
- `FontLoad`: font read/parse failures
- `Io { path, source }`: filesystem failures, with the offending path attached for actionable error messages
- `Image`: debug-mask / image decoding failures
- `Generation`: runtime algorithm failures

## ShapeSource

The shape mask is sourced from a `ShapeSource` enum rather than a single
text field, decoupling rasterization from the rest of the pipeline:

- `ShapeSource::Text { text, font_size }`: the original path — rasterize
  glyphs (auto-fit or fixed font size) into a boolean mask. Multi-line
  text is supported via embedded `\n` (the CLI exposes this through
  `--text-lines`).
- `ShapeSource::Image { path, threshold }`: load a PNG, resize to canvas
  dimensions, and threshold the alpha channel (`alpha > threshold` =
  inside) to obtain the mask.

Both variants converge on the same `BitMask` representation so all
downstream layout strategies are agnostic to the mask origin.

## TextSizeCache

Every `generate` call constructs a fresh `TextSizeCache` keyed by
`(word, font_size, rotation)`. Glyph metrics are computed lazily on
first lookup and reused across the per-call placement attempts. The
cache is intentionally per-generate rather than global — it keeps the
public API allocation-only-on-call and avoids cross-call interference
in benchmark/snapshot scenarios.

## IncrementalAvailability

`IncrementalAvailability` is the public abstraction over integral-image
based `O(1)` rectangle availability checks against the current placed
state. Each layout strategy that needs availability queries works
through this interface so the integral-image rebuild policy (full
rebuild vs. pending-rect deltas) can evolve independently of any
specific algorithm.

## BitMask

The shape mask itself is a `BitMask`: a `1-bit-per-cell` packed
representation (instead of a `Vec<bool>`). At canvas sizes typical for
production output this halves cache pressure during the hottest inner
loops (collision and integral-image scans) and is the dominant reason
the integral-image rebuild stays cheap.

## Fuzz Testing

GlyphWeave ships two complementary fuzz harnesses:

**Stable-Rust fallback** (W8 D2): `cargo test --release --test fuzz_inputs`
runs a hand-rolled harness that feeds 50 randomized inputs per public
entry point. Works on stable toolchains and runs in CI.

**Real cargo-fuzz target** (nightly + libFuzzer):

```bash
rustup install nightly
cargo install cargo-fuzz
cargo +nightly fuzz run image_mask  # CTRL-C to stop
```

The `image_mask` target feeds arbitrary bytes as a PNG-on-disk into
`build_image_mask` via `ShapeSource::Image` and runs `generate` with a
small canvas to surface panics in the image decode + mask construction
path. Crashes are written to `fuzz/artifacts/<target>/`.
