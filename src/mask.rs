use crate::core::error::GlyphWeaveError;
use crate::core::model::{CanvasConfig, Rotation};
use crate::layout::BitMask;
use fontdue::Font;
use image::{ImageBuffer, Rgba};
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
) -> BitMask {
	let mut mask = BitMask::zeros(canvas.height, canvas.width);

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
						mask.set(py, px, true);
					}
				}
			}
		}

		cursor_x += glyph_metrics.advance_width.ceil() as usize;
	}

	mask
}

pub fn build_image_mask(
	canvas: &CanvasConfig,
	image_path: &Path,
	threshold: u8,
) -> Result<BitMask, GlyphWeaveError> {
	let img = image::open(image_path).map_err(|source| GlyphWeaveError::Image {
		path: image_path.to_path_buf(),
		source,
	})?;
	let img = img.resize_exact(
		canvas.width as u32,
		canvas.height as u32,
		image::imageops::FilterType::Lanczos3,
	);
	let rgba = img.to_rgba8();
	let mut mask = BitMask::zeros(canvas.height, canvas.width);
	for y in 0..canvas.height {
		for x in 0..canvas.width {
			let p = rgba.get_pixel(x as u32, y as u32);
			let inside = p[3] > threshold;
			if inside {
				mask.set(y, x, true);
			}
		}
	}
	Ok(mask)
}

pub fn total_usable_area(mask: &BitMask) -> usize {
	mask.count_ones()
}

pub fn mask_centroid(mask: &BitMask) -> (usize, usize) {
	let mut sum_x = 0usize;
	let mut sum_y = 0usize;
	let mut count = 0usize;

	for (y, x) in mask.indexed_set() {
		sum_x += x;
		sum_y += y;
		count += 1;
	}

	if count == 0 {
		return (0, 0);
	}

	(sum_x / count, sum_y / count)
}

pub fn mask_to_image(mask: &BitMask) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
	let height = mask.nrows();
	let width = mask.ncols();
	let mut image = ImageBuffer::new(width as u32, height as u32);

	for y in 0..height {
		for x in 0..width {
			let pixel = if mask.get(y, x) {
				Rgba([255, 255, 255, 255])
			} else {
				Rgba([0, 0, 0, 0])
			};
			image.put_pixel(x as u32, y as u32, pixel);
		}
	}

	image
}

pub fn save_mask_image(mask: &BitMask, path: &Path) -> Result<(), GlyphWeaveError> {
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
		assert_eq!((mask.nrows(), mask.ncols()), (400, 800));
		assert!(total_usable_area(&mask) > 0);
	}
}
