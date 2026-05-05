use glyphweave::{
	AlgorithmKind, CanvasConfig, CloudRequest, FontSizeSpec, RenderOptions, Rotation, ShapeConfig,
	StyleConfig, WordEntry, generate, load_default_embedded_font, load_font_from_file,
	rotations_from_degrees,
};
use std::path::Path;
use std::sync::Arc;

#[test]
fn rejects_unsupported_rotation() {
	let err = rotations_from_degrees(&[360]).expect_err("rotation should be rejected");
	assert!(err.to_string().contains("unsupported rotation"));
}

#[test]
fn generate_with_same_seed_is_stable() {
	let font = load_default_embedded_font().or_else(|_| {
		let fallback = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/NotoSansSC-Regular.ttf");
		load_font_from_file(&fallback)
	});
	let font = Arc::new(font.expect("test font should load"));
	let request = CloudRequest {
		canvas: CanvasConfig {
			width: 420,
			height: 260,
			margin: 8,
		},
		shape: ShapeConfig::text("AI", FontSizeSpec::AutoFit),
		words: vec![
			WordEntry::new("rust", 2.0),
			WordEntry::new("svg", 1.0),
			WordEntry::new("mask", 1.0),
		],
		style: StyleConfig {
			font_size_range: 10..=20,
			padding: 0,
			colors: vec!["#000".to_string()],
			rotations: vec![Rotation::Deg0],
		},
		algorithm: AlgorithmKind::FastGrid,
		ratio_threshold: 0.3,
		max_try_count: 1200,
		seed: Some(99),
		font,
		render: RenderOptions {
			show_progress: false,
			debug_mask_out: None,
		},
	};

	let a = generate(request.clone()).expect("generation should succeed");
	let b = generate(request).expect("generation should succeed");
	assert_eq!(a.svg, b.svg);
}
