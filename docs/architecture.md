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
- `Io` / `Image`: filesystem and debug-mask failures
- `Generation`: runtime algorithm failures
