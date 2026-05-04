use crate::core::error::GlyphWeaveError;
use crate::core::model::{CanvasConfig, Rotation};
use fontdue::Font;
use image::{ImageBuffer, Rgba};
use ndarray::Array2;
use std::path::Path;

pub fn calculate_text_size(
	text: &str,
	font: &Font,
	font_size: usize,
	padding: usize,
	rotation: Rotation,
) -> (usize, usize) {
	let metrics: Vec<_> = text
		.chars()
		.map(|c| font.metrics(c, font_size as f32))
		.collect();
	let width = metrics.iter().map(|m| m.advance_width).sum::<f32>().ceil() as usize + 2 * padding;
	let height = metrics.iter().map(|m| m.height).max().unwrap_or(0) + 2 * padding;

	match rotation {
		Rotation::Deg0 => (width, height),
		Rotation::Deg90 => (height, width),
	}
}

pub fn calculate_auto_font_size(canvas: &CanvasConfig, text: &str, font: &Font) -> usize {
	let available_width = canvas.width.saturating_sub(2 * canvas.margin);
	let available_height = canvas.height.saturating_sub(2 * canvas.margin);

	let mut low = 1usize;
	let mut high = available_height.max(1);
	let mut best = 1usize;

	while low <= high {
		let mid = low + (high - low) / 2;
		let (w, h) = calculate_text_size(text, font, mid, 0, Rotation::Deg0);
		if w <= available_width && h <= available_height {
			best = mid;
			low = mid + 1;
		} else {
			if mid == 0 {
				break;
			}
			high = mid.saturating_sub(1);
		}
	}

	best
}

pub fn build_shape_mask(
	canvas: &CanvasConfig,
	text: &str,
	font: &Font,
	font_size: usize,
) -> Array2<bool> {
	let mut mask = Array2::from_elem((canvas.height, canvas.width), false);

	let metrics: Vec<_> = text
		.chars()
		.map(|c| font.metrics(c, font_size as f32))
		.collect();
	let text_width = metrics.iter().map(|m| m.advance_width).sum::<f32>().ceil() as usize;
	let text_height = metrics.iter().map(|m| m.height).max().unwrap_or(0);

	let offset_x = canvas.margin
		+ (canvas
			.width
			.saturating_sub(2 * canvas.margin)
			.saturating_sub(text_width))
			/ 2;
	let offset_y = canvas.margin
		+ (canvas
			.height
			.saturating_sub(2 * canvas.margin)
			.saturating_sub(text_height))
			/ 2;

	let mut cursor_x = offset_x;

	for (ch, glyph_metrics) in text.chars().zip(metrics.iter()) {
		let (raster_metrics, bitmap) = font.rasterize(ch, font_size as f32);

		for y in 0..raster_metrics.height {
			for x in 0..raster_metrics.width {
				let pixel = bitmap[y * raster_metrics.width + x];
				if pixel > 127 {
					let px = cursor_x + x;
					let py = offset_y + y;
					if px < canvas.width && py < canvas.height {
						mask[[py, px]] = true;
					}
				}
			}
		}

		cursor_x += glyph_metrics.advance_width.ceil() as usize;
	}

	mask
}

pub fn total_usable_area(mask: &Array2<bool>) -> usize {
	mask.iter().filter(|&&value| value).count()
}

pub fn mask_centroid(mask: &Array2<bool>) -> (usize, usize) {
	let mut sum_x = 0usize;
	let mut sum_y = 0usize;
	let mut count = 0usize;

	for ((y, x), value) in mask.indexed_iter() {
		if *value {
			sum_x += x;
			sum_y += y;
			count += 1;
		}
	}

	if count == 0 {
		return (0, 0);
	}

	(sum_x / count, sum_y / count)
}

pub fn mask_to_image(mask: &Array2<bool>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
	let (height, width) = mask.dim();
	let mut image = ImageBuffer::new(width as u32, height as u32);

	for ((y, x), occupied) in mask.indexed_iter() {
		let pixel = if *occupied {
			Rgba([255, 255, 255, 255])
		} else {
			Rgba([0, 0, 0, 0])
		};
		image.put_pixel(x as u32, y as u32, pixel);
	}

	image
}

pub fn save_mask_image(mask: &Array2<bool>, path: &Path) -> Result<(), GlyphWeaveError> {
	let image = mask_to_image(mask);
	image.save(path).map_err(|source| GlyphWeaveError::Image {
		path: path.to_path_buf(),
		source,
	})?;
	Ok(())
}

#[cfg(all(test, feature = "embedded_fonts"))]
mod tests {
	use super::*;

	#[test]
	fn auto_font_size_and_mask_are_valid() {
		let font = crate::font::load_default_embedded_font().expect("embedded font should load");
		let canvas = CanvasConfig {
			width: 800,
			height: 400,
			margin: 20,
		};
		let size = calculate_auto_font_size(&canvas, "HELLO", &font);
		assert!(size > 0);

		let mask = build_shape_mask(&canvas, "HELLO", &font, size);
		assert_eq!(mask.dim(), (400, 800));
		assert!(total_usable_area(&mask) > 0);
	}
}
