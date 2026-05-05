#![allow(dead_code)]

use glyphweave::{
	AlgorithmKind, CanvasConfig, CloudRequest, CloudResult, FontSizeSpec, RenderOptions, Rotation,
	ShapeConfig, StyleConfig, WordEntry, generate, load_default_embedded_font, load_font_from_file,
	mask::{build_shape_mask, calculate_auto_font_size, calculate_text_size},
};
use std::path::Path;
use std::sync::Arc;

pub fn sample_words() -> Vec<WordEntry> {
	vec![
		WordEntry::new("rust", 3.0),
		WordEntry::new("cloud", 2.0),
		WordEntry::new("layout", 1.8),
		WordEntry::new("mask", 1.6),
		WordEntry::new("svg", 1.5),
		WordEntry::new("engine", 1.2),
		WordEntry::new("algorithm", 1.1),
		WordEntry::new("seed", 1.0),
		WordEntry::new("score", 1.0),
		WordEntry::new("shape", 1.0),
	]
}

pub fn build_request(algorithm: AlgorithmKind) -> CloudRequest {
	let font = load_default_embedded_font().or_else(|_| {
		let fallback = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/NotoSansSC-Regular.ttf");
		load_font_from_file(&fallback)
	});
	let font = font.expect("test font should load");

	CloudRequest {
		canvas: CanvasConfig {
			width: 360,
			height: 220,
			margin: 8,
		},
		shape: ShapeConfig {
			text: "RUST".to_string(),
			font_size: FontSizeSpec::AutoFit,
		},
		words: sample_words(),
		style: StyleConfig {
			font_size_range: 10..=20,
			padding: 0,
			colors: vec![
				"#1D4ED8".to_string(),
				"#DB2777".to_string(),
				"#059669".to_string(),
				"#EA580C".to_string(),
			],
			rotations: vec![Rotation::Deg0],
		},
		algorithm,
		ratio_threshold: 0.2,
		max_try_count: 260,
		seed: Some(20260305),
		font: Arc::new(font),
		render: RenderOptions {
			show_progress: false,
			debug_mask_out: None,
		},
	}
}

pub fn normalize_svg(svg: &str) -> String {
	svg.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn generate_normalized(request: CloudRequest) -> (CloudRequest, CloudResult, String) {
	let result = generate(request.clone()).expect("generation should succeed");
	let normalized = normalize_svg(&result.svg);
	(request, result, normalized)
}

pub fn assert_placement_constraints(request: &CloudRequest, result: &CloudResult) {
	let shape_font_size = match request.shape.font_size {
		FontSizeSpec::Fixed(size) => size,
		FontSizeSpec::AutoFit => {
			calculate_auto_font_size(&request.canvas, &request.shape.text, request.font.as_ref())
		}
	};

	let shape_mask = build_shape_mask(
		&request.canvas,
		&request.shape.text,
		request.font.as_ref(),
		shape_font_size,
	);

	for (i, placement) in result.placements.iter().enumerate() {
		let (w, h) = calculate_text_size(
			&placement.word,
			request.font.as_ref(),
			placement.font_size,
			request.style.padding,
			placement.rotation,
		);

		assert!(
			placement.x + w <= request.canvas.width,
			"placement {i} exceeds width"
		);
		assert!(
			placement.y + h <= request.canvas.height,
			"placement {i} exceeds height"
		);

		for dy in 0..h {
			for dx in 0..w {
				assert!(
					shape_mask.get(placement.y + dy, placement.x + dx),
					"placement {i} leaves shape at ({}, {})",
					placement.x + dx,
					placement.y + dy
				);
			}
		}

		assert!(
			request.style.colors.contains(&placement.color),
			"placement {i} uses color outside configured palette"
		);

		assert!(
			request.style.font_size_range.contains(&placement.font_size),
			"placement {i} font size out of range"
		);
	}

	for i in 0..result.placements.len() {
		let a = &result.placements[i];
		let (aw, ah) = calculate_text_size(
			&a.word,
			request.font.as_ref(),
			a.font_size,
			request.style.padding,
			a.rotation,
		);

		for j in (i + 1)..result.placements.len() {
			let b = &result.placements[j];
			let (bw, bh) = calculate_text_size(
				&b.word,
				request.font.as_ref(),
				b.font_size,
				request.style.padding,
				b.rotation,
			);

			let overlap = a.x < b.x + bw && a.x + aw > b.x && a.y < b.y + bh && a.y + ah > b.y;
			assert!(!overlap, "placements {i} and {j} overlap");
		}
	}
}
