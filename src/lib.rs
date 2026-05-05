pub mod core;
pub mod font;
pub mod layout;
pub mod mask;
pub mod render;

mod embedded_fonts;

use crate::core::error::GlyphWeaveError;
use crate::layout::{LayoutRequest, TextSizeCache, strategy_for};
use crate::mask::{
	build_image_mask, build_shape_mask, calculate_auto_font_size, save_mask_image,
	total_usable_area,
};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::time::Instant;

pub use crate::core::model::{
	AlgorithmKind, CanvasConfig, CloudPlacement, CloudRequest, CloudResult, CloudStats,
	FontSizeSpec, RenderOptions, Rotation, ShapeConfig, ShapeSource, StyleConfig, WordEntry,
};
pub use crate::font::{
	discover_system_font_candidates, load_default_embedded_font, load_font_from_file,
	load_system_font,
};

pub fn generate(request: CloudRequest) -> Result<CloudResult, GlyphWeaveError> {
	request.validate()?;

	let started_at = Instant::now();

	let (shape_mask, shape_font_size) = match &request.shape.source {
		ShapeSource::Text { text, font_size } => {
			let resolved = match font_size {
				FontSizeSpec::Fixed(size) => *size,
				FontSizeSpec::AutoFit => {
					calculate_auto_font_size(&request.canvas, text, request.font.as_ref())
				}
			};
			let mask = build_shape_mask(&request.canvas, text, request.font.as_ref(), resolved);
			(mask, resolved)
		}
		ShapeSource::Image { path, threshold } => {
			let mask = build_image_mask(&request.canvas, path, *threshold)?;
			(mask, 0)
		}
	};

	if let Some(path) = &request.render.debug_mask_out {
		save_mask_image(&shape_mask, path)?;
	}

	let total_area = total_usable_area(&shape_mask);
	if total_area == 0 {
		return Err(GlyphWeaveError::Generation(
			"shape mask is empty; try a different text/font/canvas combination".to_string(),
		));
	}

	let used_seed = request.seed.unwrap_or_else(rand::random::<u64>);
	let mut rng = StdRng::seed_from_u64(used_seed);

	let layout_req = LayoutRequest {
		mask: &shape_mask,
		words: &request.words,
		style: &request.style,
		font: request.font.as_ref(),
		ratio_threshold: request.ratio_threshold,
		max_try_count: request.max_try_count,
		show_progress: request.render.show_progress,
		text_size_cache: TextSizeCache::new(),
	};

	let strategy = strategy_for(request.algorithm);
	let layout_result = strategy.place(&layout_req, &mut rng)?;

	let svg = render::render_svg(
		&request.canvas,
		&layout_result.placements,
		request.font.as_ref(),
		&font::font_family_name(request.font.as_ref()),
	);

	let placed_words = layout_result.placements.len();
	let fill_ratio = layout_result.used_area as f32 / total_area as f32;

	Ok(CloudResult {
		svg,
		placements: layout_result.placements,
		stats: CloudStats {
			seed: used_seed,
			shape_font_size,
			total_usable_area: total_area,
			used_area: layout_result.used_area,
			fill_ratio,
			attempts: layout_result.attempts,
			internal_evaluations: layout_result.internal_evaluations,
			placed_words,
			elapsed_ms: started_at.elapsed().as_millis(),
		},
	})
}

pub fn rotations_from_degrees(values: &[u16]) -> Result<Vec<Rotation>, GlyphWeaveError> {
	let mut rotations = Vec::new();
	for value in values {
		if *value >= 360 {
			return Err(GlyphWeaveError::InvalidConfig(format!(
				"unsupported rotation '{value}', must be in 0..360"
			)));
		}
		let rotation = Rotation(*value);
		if !rotations.contains(&rotation) {
			rotations.push(rotation);
		}
	}

	if rotations.is_empty() {
		rotations.push(Rotation::Deg0);
	}

	Ok(rotations)
}

#[cfg(all(test, feature = "embedded_fonts"))]
mod tests {
	use super::*;
	use std::sync::Arc;

	#[test]
	fn generation_is_deterministic_with_seed() {
		let font = load_default_embedded_font().expect("embedded font should be available");
		let request = CloudRequest {
			canvas: CanvasConfig {
				width: 480,
				height: 320,
				margin: 12,
			},
			shape: ShapeConfig::text("AI", FontSizeSpec::AutoFit),
			words: vec![
				WordEntry::new("Rust", 2.0),
				WordEntry::new("Cloud", 1.0),
				WordEntry::new("Speed", 1.5),
			],
			style: StyleConfig {
				font_size_range: 12..=24,
				padding: 0,
				colors: vec!["#111111".to_string(), "#228833".to_string()],
				rotations: vec![Rotation::Deg0],
			},
			algorithm: AlgorithmKind::FastGrid,
			ratio_threshold: 0.25,
			max_try_count: 800,
			seed: Some(1234),
			font: Arc::new(font),
			render: RenderOptions {
				show_progress: false,
				debug_mask_out: None,
			},
		};

		let result_a = generate(request.clone()).expect("generation should succeed");
		let result_b = generate(request).expect("generation should succeed");

		assert_eq!(result_a.svg, result_b.svg);
		assert_eq!(result_a.stats.seed, 1234);
	}
}
