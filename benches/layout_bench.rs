use criterion::{Criterion, black_box, criterion_group, criterion_main};
use glyphweave::{
	AlgorithmKind, CanvasConfig, CloudRequest, FontSizeSpec, RenderOptions, ShapeConfig,
	StyleConfig, WordEntry, generate, load_default_embedded_font,
};
use std::sync::Arc;

fn bench_layouts(c: &mut Criterion) {
	let font = Arc::new(load_default_embedded_font().expect("embedded font should load"));
	let words = vec![
		WordEntry::new("rust", 3.0),
		WordEntry::new("layout", 2.0),
		WordEntry::new("mask", 2.0),
		WordEntry::new("svg", 1.5),
		WordEntry::new("cloud", 1.0),
	];

	let mut group = c.benchmark_group("layout");
	group.sample_size(10);

	for (algorithm, label, max_tries, ratio) in [
		(
			AlgorithmKind::RandomBaseline,
			"RandomBaseline",
			5_000_usize,
			0.75_f32,
		),
		(AlgorithmKind::FastGrid, "FastGrid", 5_000, 0.75),
		(AlgorithmKind::SpiralGreedy, "SpiralGreedy", 200, 0.50),
		(
			AlgorithmKind::SimulatedAnnealing,
			"SimulatedAnnealing",
			1_000,
			0.50,
		),
		(AlgorithmKind::Mcts, "MCTS", 200, 0.50),
	] {
		group.bench_function(label, |b| {
			let words = words.clone();
			let font = Arc::clone(&font);
			b.iter(|| {
				let req = CloudRequest {
					canvas: CanvasConfig {
						width: 600,
						height: 400,
						margin: 12,
					},
					shape: ShapeConfig::text("RUST", FontSizeSpec::AutoFit),
					words: words.clone(),
					style: StyleConfig {
						font_size_range: 12..=24,
						padding: 0,
						colors: vec!["#111111".to_string(), "#2277aa".to_string()],
						rotations: vec![glyphweave::core::model::Rotation::Deg0],
					},
					algorithm,
					ratio_threshold: ratio,
					max_try_count: max_tries,
					seed: Some(42),
					font: Arc::clone(&font),
					render: RenderOptions {
						show_progress: false,
						debug_mask_out: None,
					},
				};
				black_box(generate(req).expect("generation should succeed"));
			})
		});
	}

	group.finish();
}

criterion_group!(benches, bench_layouts);
criterion_main!(benches);
