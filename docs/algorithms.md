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

## 2. SpiralGreedy (Archimedean)

Goal: produce visually coherent center-out layouts.

Key ideas:

- start from mask centroid
- traverse a discretized **Archimedean spiral** `r(θ) = θ` with adaptive
  step `Δθ ≈ 1/(2r)` so neighboring samples are ~1 cell apart, giving
  isotropic angular density (matches ShapeWordle / Wordle baseline)
- prefer larger font sizes first, then fallback

Best for:
- stable visual structure
- center-focused compositions
- direct comparison with shape-aware spiral methods (e.g. ShapeWordle's
  distance-field-driven Archimedean spiral)

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
`90`-only restriction has been lifted; `Rotation::Deg0` and
`Rotation::Deg90` constants remain for the common case.
