mod support;

use glyphweave::{
	AlgorithmKind, CanvasConfig, CloudRequest, FontSizeSpec, RenderOptions, Rotation, ShapeConfig,
	StyleConfig, WordEntry, generate, load_default_embedded_font, load_font_from_file,
};
use proptest::prelude::*;
use std::path::Path;
use std::sync::Arc;
use support::assert_placement_constraints;

fn arb_algorithm() -> impl Strategy<Value = AlgorithmKind> {
	prop_oneof![
		Just(AlgorithmKind::FastGrid),
		Just(AlgorithmKind::RandomBaseline),
		Just(AlgorithmKind::SpiralGreedy),
		Just(AlgorithmKind::Mcts),
		Just(AlgorithmKind::SimulatedAnnealing),
	]
}

fn arb_word() -> impl Strategy<Value = WordEntry> {
	("[a-z]{2,8}", 0.5f32..3.0f32).prop_map(|(text, weight)| WordEntry::new(text, weight))
}

fn arb_request(font: Arc<fontdue::Font>) -> impl Strategy<Value = CloudRequest> {
	(
		100usize..400,
		80usize..200,
		prop::collection::vec(arb_word(), 2..6),
		arb_algorithm(),
		any::<u64>(),
	)
		.prop_map(move |(w, h, words, algo, seed)| {
			let max_tries = match algo {
				AlgorithmKind::Mcts => 30,
				AlgorithmKind::SimulatedAnnealing => 100,
				_ => 200,
			};
			CloudRequest {
				canvas: CanvasConfig {
					width: w,
					height: h,
					margin: 4,
				},
				shape: ShapeConfig::text("AI", FontSizeSpec::AutoFit),
				words,
				style: StyleConfig {
					font_size_range: 8..=14,
					padding: 0,
					colors: vec!["#111".into(), "#222".into()],
					rotations: vec![Rotation::Deg0],
				},
				algorithm: algo,
				ratio_threshold: 0.3,
				max_try_count: max_tries,
				seed: Some(seed),
				font: Arc::clone(&font),
				render: RenderOptions {
					show_progress: false,
					debug_mask_out: None,
				},
			}
		})
}

fn load_test_font() -> fontdue::Font {
	load_default_embedded_font()
		.or_else(|_| {
			load_font_from_file(
				Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/NotoSansSC-Regular.ttf"),
			)
		})
		.expect("font should load")
}

proptest! {
	#![proptest_config(ProptestConfig {
		cases: 16,
		max_shrink_iters: 32,
		..ProptestConfig::default()
	})]

	#[test]
	fn placements_satisfy_invariants(
		request in arb_request(Arc::new(load_test_font()))
	) {
		let result = generate(request.clone()).expect("generation should succeed");
		assert_placement_constraints(&request, &result);
	}
}
