mod support;

use glyphweave::AlgorithmKind;
use std::fs;
use std::path::PathBuf;
use support::{assert_placement_constraints, build_request_with_seed, generate_normalized};

#[test]
fn layout_svg_snapshots_are_stable() {
	let cases = [
		(AlgorithmKind::FastGrid, "fast-grid"),
		(AlgorithmKind::RandomBaseline, "random-baseline"),
		(AlgorithmKind::SpiralGreedy, "spiral-greedy"),
		(AlgorithmKind::Mcts, "mcts"),
		(AlgorithmKind::SimulatedAnnealing, "simulated-annealing"),
	];

	let seeds = [20260305u64, 7, 99];

	let update = std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1");

	for (algorithm, label) in cases {
		for &seed in &seeds {
			let request = build_request_with_seed(algorithm, seed);
			let (request, result, normalized_svg) = generate_normalized(request);
			assert_placement_constraints(&request, &result);

			let snapshot_path = PathBuf::from(format!("tests/golden/{label}-seed{seed}.svg.snap"));

			if update {
				fs::write(&snapshot_path, &normalized_svg).unwrap_or_else(|err| {
					panic!("failed to write {}: {err}", snapshot_path.display())
				});
				continue;
			}

			let expected = fs::read_to_string(&snapshot_path).unwrap_or_else(|err| {
				panic!(
					"missing snapshot {} ({err}); run UPDATE_GOLDEN=1 cargo test --test regression_snapshots",
					snapshot_path.display()
				)
			});

			assert_eq!(
				normalized_svg, expected,
				"snapshot mismatch for {label} seed {seed}; if intentional, run UPDATE_GOLDEN=1 cargo test --test regression_snapshots"
			);
		}
	}
}
