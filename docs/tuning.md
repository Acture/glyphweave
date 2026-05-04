# Tuning Guide

## Recommended Starting Point

- algorithm: `fast-grid`
- ratio: `0.85`
- max-tries: `10000`
- word-size-range: `10,30`
- rotations: `0`

## Parameters

### `--ratio`

Target fill ratio (`0.0..=1.0`).

- higher ratio => denser shape, slower runtime
- lower ratio => faster runtime, more empty regions

### `--max-tries`

Upper bound for placement attempts.

- increase for larger canvases or aggressive ratio goals
- when runtime is too high, lower this first

### `--word-size-range`

Controls typography hierarchy.

- wider range creates stronger contrast
- very large upper bound may reduce fit success near boundaries

### `--word-padding`

Pixels of padding added around each placed word's bounding box.

- default `0` packs words tightly (best for fill ratio, can look dense)
- `1`–`3` improves visual separation in dense clouds at the cost of fewer placements
- equivalent TOML field: `word_padding`

### `--rotations`

`0` or `0,90`.

- adding `90` can increase fit opportunities in narrow areas
- may reduce reading consistency for some datasets

### `--seed`

Use fixed seed in CI/regression tests.

- deterministic output enables snapshot-based verification
- random seed is useful for exploratory design generation

## Performance Playbook

1. Use `fast-grid`.
2. Start with `ratio=0.8` and gradually increase.
3. Keep `rotations=0` unless shape has many narrow gaps.
4. Increase `max-tries` only after tuning ratio and size range.
