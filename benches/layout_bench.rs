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

	for algorithm in [
		AlgorithmKind::RandomBaseline,
		AlgorithmKind::FastGrid,
		AlgorithmKind::SpiralGreedy,
	] {
		group.bench_function(format!("{:?}", algorithm), |b| {
			let words = words.clone();
			let font = Arc::clone(&font);
			b.iter(|| {
				let req = CloudRequest {
					canvas: CanvasConfig {
						width: 900,
						height: 520,
						margin: 12,
					},
					shape: ShapeConfig::text("RUST", FontSizeSpec::AutoFit),
					words: words.clone(),
					style: StyleConfig {
						font_size_range: 12..=28,
						padding: 0,
						colors: vec!["#111111".to_string(), "#2277aa".to_string()],
						rotations: vec![glyphweave::core::model::Rotation::Deg0],
					},
					algorithm,
					ratio_threshold: 0.75,
					max_try_count: 5_000,
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
