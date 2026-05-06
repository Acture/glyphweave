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

- default `2` provides comfortable visual separation out of the box
- `0` packs words tightly (best for fill ratio, can look dense)
- `1`–`3` improves visual separation in dense clouds at the cost of fewer placements
- equivalent TOML field: `word_padding`

### `--rotations`

Any comma-separated integer angles in `0..=360` degrees. Examples:
`0`, `0,90`, `0,30,60,90`.

- adding `90` can increase fit opportunities in narrow areas
- non-orthogonal angles (e.g. `0,45`) add typographic variety but slow placement
- may reduce reading consistency for some datasets

### `--seed`

Use fixed seed in CI/regression tests.

- deterministic output enables snapshot-based verification
- random seed is useful for exploratory design generation

## Algorithm-specific Budgets

`--max-tries` does not mean the same thing across algorithms. Reasonable
starting budgets:

| Algorithm | Recommended `max-tries` | Notes |
|---|---:|---|
| `fast-grid` | 5_000 – 10_000 | Default production choice; integral image keeps inner cost O(1) |
| `random-baseline` | 5_000 – 10_000 | Cheap per-attempt; bound to keep runtime predictable |
| `simulated-annealing` | 1_000 – 2_000 | One sample per attempt; raise temperature/cooling parameters before increasing budget |
| `spiral-greedy` | **200 – 500** | Each attempt walks an Archimedean offset table of ~10⁵–10⁶ cells per font-size × rotation combination — keep the budget small |
| `mcts` | 100 – 500 | Each attempt expands a search tree with ~10⁴ rollout evaluations |

When you change algorithms, recheck this table — copying `max-tries=10000`
from `fast-grid` to `mcts` or `spiral-greedy` will make runs hundreds of
times slower for little quality gain.

> **SpiralGreedy perf note.** SpiralGreedy walks an offset table sized
> roughly `max(canvas_w, canvas_h)²` cells per font-size × rotation
> combination. For a 1920×1080 canvas with ~30 distinct font sizes and
> 2 rotations, a single attempt can sample over 200M positions. Treat
> its `max-tries` budget as small, prefer a tighter canvas, and avoid
> `--rotations` unless the shape really demands it.

## Reading `internal_evaluations`

`CloudStats.internal_evaluations` reports the true number of
placement-attempt evaluations performed inside the chosen layout
strategy. Unlike `attempts` (a coarse outer-loop counter), it lets you
compare the *real* computational work between algorithms on the same
input — e.g. `fast-grid` vs. `mcts` on the same shape and word list.
Use it as a proxy for runtime when comparing algorithm choices, and as
a regression metric for performance-sensitive changes.

## Performance Playbook

1. Use `fast-grid`.
2. Start with `ratio=0.8` and gradually increase.
3. Keep `rotations=0` unless shape has many narrow gaps.
4. Increase `max-tries` only after tuning ratio and size range — and
   keep the per-algorithm budget table above in mind.
5. Track `internal_evaluations` across runs to detect performance
   regressions independent of wall-clock noise.
