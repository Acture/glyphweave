# Algorithms

## 1. FastGrid (default)

Goal: maximize speed while preserving fill quality.

Key ideas:

- maintain a sampled candidate position pool instead of scanning all pixels every iteration
- use integral image checks for O(1) rectangle availability against stable mask state
- keep incremental pending-rect collision checks between integral rebuilds

Best for:

- medium/large canvases
- larger word lists
- production default

## 2. SpiralGreedy (rectangular spiral)

Goal: produce visually coherent center-out layouts on an axis-aligned grid.

Key ideas:

- start from mask centroid
- traverse a **rectangular spiral** of cell offsets (1 right, 1 down, 2 left, 2 up, 3 right, ...) — *not* a true Archimedean spiral. Trade-off: faster to enumerate but with anisotropic density (dense along axes, sparse along diagonals)
- prefer larger font sizes first, then fallback

Best for:

- stable visual structure
- center-focused compositions
- baseline against shape-aware spiral methods (e.g. ShapeWordle's distance-field-driven Archimedean spiral)

Limitation: ~195k offset cells are pre-tabulated; for masks where most candidate slots are far from the center, this becomes the dominant cost (see internal_evaluations stats).

## 3. RandomBaseline

Goal: simple baseline for comparison and regression.

Key ideas:

- random point sampling from currently available mask area
- descending font-size fit check at each sampled point

Best for:

- algorithm A/B baseline
- correctness and compatibility checks

## 4. MCTS

Goal: use Monte Carlo search to pick stronger placements each step.

Key ideas:

- sample multiple candidate placements as root children
- run UCB-based selection and **diff-and-rollback rollout** for each
  child: simulated placements are applied to the live mask /
  availability state and then explicitly rolled back via a recorded
  diff, instead of cloning the whole board each rollout. This keeps
  rollouts allocation-light and makes deeper search affordable.
- choose the child with best estimated reward (usable-area gain)

Best for:

- quality-oriented generation
- scenarios where runtime can be traded for better packing decisions

Tip: keep `--max-tries` modest (~200). Each "try" expands the search
tree and runs rollouts, so budgets sized for `fast-grid` will produce
runaway runtimes here.

## 5. SimulatedAnnealing

Goal: stochastic optimization with probabilistic acceptance.

Key ideas:

- sample valid placement candidates
- score candidates by area gain and word weight
- accept worse moves with temperature-dependent probability

Best for:

- escaping strict greedy behavior
- obtaining diverse layout outcomes under fixed constraints

## Rotation Support

Rotations accept any integer angles in `0..=360` degrees (CLI:
`--rotations 0,30,60,90`; library: `Rotation(u16)`). The earlier `0` /
`90`-only restriction has been lifted; `Rotation::ZERO` and
`Rotation::NINETY` constants remain for the common case.
