mod support;

use glyphweave::{AlgorithmKind, generate};
use support::build_request;

#[test]
#[ignore = "performance checks are noisy in shared environments; run manually in CI perf job"]
fn fast_grid_internal_evaluations_should_stay_bounded() {
	let request = build_request(AlgorithmKind::FastGrid);
	let result = generate(request).expect("fast-grid generation should succeed");

	assert!(
		result.stats.internal_evaluations <= 125_000,
		"fast-grid regression: internal_evaluations={}",
		result.stats.internal_evaluations
	);
}
