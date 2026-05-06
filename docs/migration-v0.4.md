# Migration Guide: v0.3 → v0.4

This page covers the breaking changes in 0.4.0. Library and CLI users
upgrading from 0.3.x should review each section.

## TL;DR

| Change | Affects | Action |
|---|---|---|
| TOML config rejects unknown fields | Config files | Remove or correct typo'd fields |
| `GlyphWeaveError::Io` / `Image` is now a struct variant | Library callers matching on error | Update match patterns to `{ path, source }` |
| `ShapeConfig` now wraps `ShapeSource` enum | Library callers building `CloudRequest` | Use `ShapeConfig::text(...)` / `ShapeConfig::image(...)` |
| `Rotation` is now `struct(u16)`, supports any 0..360 | Library callers matching on `Rotation` | Switch to the `.degrees()` accessor |
| `CloudStats.elapsed_ms` (u128) renamed to `.elapsed` (`Duration`) | Library callers reading the field | Call `.as_millis()` to recover the old value |

JSON serialization of `CloudStats` retains the old key and format for
backward compatibility (`"elapsed_ms": 1234` in JSON output).

## 1. TOML config rejects unknown fields

The TOML config loader now uses `#[serde(deny_unknown_fields)]`, so any
typo or stale field in a config file produces a hard error instead of
being silently ignored.

**Before** — a misspelled key was dropped silently and the run used the
default value:

```toml
[render]
font_szie = 96   # typo, silently ignored
```

**After** — the same file fails to load:

```text
Error: failed to parse config: unknown field `font_szie`,
       expected one of `font_size`, `palette`, ...
```

**Action:** rename or remove any unknown fields. If you intentionally
keep extra metadata in the file, move it into a `[meta]` table the
loader does not parse.

## 2. `GlyphWeaveError::Io` / `Image` are struct variants

`Io` and `Image` are now struct variants that carry the offending path
in addition to the underlying source error. The blanket `#[from]
io::Error` conversion has been removed because it could not attach a
meaningful path.

**Before:**

```rust
match err {
    GlyphWeaveError::Io(e)    => eprintln!("io: {}", e),
    GlyphWeaveError::Image(e) => eprintln!("image: {}", e),
    other => return Err(other),
}
```

**After:**

```rust
match err {
    GlyphWeaveError::Io { path, source } => {
        eprintln!("io at {}: {}", path.display(), source);
    }
    GlyphWeaveError::Image { path, source } => {
        eprintln!("image at {}: {}", path.display(), source);
    }
    other => return Err(other),
}
```

When constructing the error yourself, build the struct variant with the
relevant path:

```rust
return Err(GlyphWeaveError::Io {
    path: cfg_path.to_path_buf(),
    source: io_err,
});
```

If you previously relied on `?` against `io::Error`, attach the path at
the call site (for example via `.map_err(|source| GlyphWeaveError::Io { path, source })`).

## 3. `ShapeConfig` wraps `ShapeSource` enum

`ShapeConfig` no longer holds the text/font fields directly. It now
contains a `ShapeSource` enum so that 0.4 can support image-mask shapes
in addition to text.

**Before:**

```rust
let req = CloudRequest {
    shape: ShapeConfig {
        text: "AI".into(),
        font_size: FontSizeSpec::AutoFit,
    },
    ..req
};
```

**After:**

```rust
let req = CloudRequest {
    shape: ShapeConfig::text("AI", FontSizeSpec::AutoFit),
    ..req
};
```

The new image-mask path uses the sibling constructor:

```rust
let req = CloudRequest {
    shape: ShapeConfig::image(PathBuf::from("logo.png"), 127),
    ..req
};
```

If you need to inspect the shape, match on `shape.source`:

```rust
match &req.shape.source {
    ShapeSource::Text { text, font_size } => { /* ... */ }
    ShapeSource::Image { path, threshold } => { /* ... */ }
}
```

## 4. `Rotation` is a newtype with arbitrary degrees

`Rotation` is now a `pub struct Rotation(pub u16)` that accepts any
angle in `0..360`. `Deg0` and `Deg90` are kept as associated constants,
so most call sites that *construct* a `Rotation` keep working — but
`match` arms that treat it as an enum need to be updated.

**Before:**

```rust
match placement.rotation {
    Rotation::Deg0  => /* 0° branch */,
    Rotation::Deg90 => /* 90° branch */,
}
```

**After:**

```rust
match placement.rotation.degrees() {
    0  => /* 0° branch */,
    90 => /* 90° branch */,
    deg => /* any other angle in 0..360 */,
}
```

Constructing rotations is unchanged for the cardinal cases:

```rust
let r = Rotation::Deg90;          // still compiles (associated const)
let r = Rotation::new(45).unwrap(); // new: arbitrary angle
```

This unlocks finer-grained rotation sets on the CLI (for example
`--rotations 0,30,60,90,120`) and lets library callers feed any angle
into the planner.

## 5. `CloudStats.elapsed`: `Duration`

`CloudStats.elapsed_ms: u128` has been replaced with
`CloudStats.elapsed: std::time::Duration`. The `Duration` type is
strictly more expressive (millis, micros, secs_f32, …) and avoids
accidental precision loss.

**Before:**

```rust
println!("took {} ms", result.stats.elapsed_ms);
```

**After:**

```rust
println!("took {} ms", result.stats.elapsed.as_millis());
// or: result.stats.elapsed.as_secs_f32(), .as_micros(), ...
```

**JSON compatibility.** The serializer keeps the old field name and
format, so any downstream consumer that parses the JSON output still
sees:

```json
{ "elapsed_ms": 1234 }
```

Only the in-memory Rust field changed.

## See also

- [docs/library-api.md](library-api.md) — current public API surface
- [CHANGELOG.md](../CHANGELOG.md) — full 0.4.0 changelog
- [docs/migration-v0.2.md](migration-v0.2.md) — earlier migration guide
