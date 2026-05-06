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

	let deg = rotation.degrees();
	if deg == 0 {
		return (width, height);
	}
	if deg == 90 || deg == 270 {
		return (height, width);
	}
	if deg == 180 {
		return (width, height);
	}
	let radians = rotation.radians();
	let cos = radians.cos().abs();
	let sin = radians.sin().abs();
	let w = width as f32;
	let h = height as f32;
	let new_w = (w * cos + h * sin).ceil() as usize;
	let new_h = (w * sin + h * cos).ceil() as usize;
	(new_w, new_h)
}

pub fn calculate_auto_font_size(canvas: &CanvasConfig, text: &str, font: &Font) -> usize {
	let lines: Vec<&str> = text.split('\n').collect();
	if lines.is_empty() {
		return 1;
	}

	let available_width = canvas.width.saturating_sub(2 * canvas.margin);
	let available_height = canvas.height.saturating_sub(2 * canvas.margin);

	let mut low = 1usize;
	let mut high = available_height.max(1);
	let mut best = 1usize;

	while low <= high {
		let mid = low + (high - low) / 2;
		debug_assert!(mid >= 1, "auto-fit binary search produced mid < 1");
		let fits = if lines.len() == 1 {
			let (w, h) = calculate_text_size(lines[0], font, mid, 0, Rotation::Deg0);
			w <= available_width && h <= available_height
		} else {
			let max_w = lines
				.iter()
				.map(|line| calculate_text_size(line, font, mid, 0, Rotation::Deg0).0)
				.max()
				.unwrap_or(0);
			let line_height = if let Some(m) = font.horizontal_line_metrics(mid as f32) {
				m.new_line_size.ceil() as usize
			} else {
				mid + mid / 5
			};
			let total_h = line_height * lines.len();
			max_w <= available_width && total_h <= available_height
		};
		if fits {
			best = mid;
			low = mid + 1;
		} else {
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
	let lines: Vec<&str> = text.split('\n').collect();

	if lines.len() <= 1 {
		let line = lines.first().copied().unwrap_or("");
		let metrics: Vec<_> = line
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
		for (ch, glyph_metrics) in line.chars().zip(metrics.iter()) {
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
		return mask;
	}

	let line_height = if let Some(m) = font.horizontal_line_metrics(font_size as f32) {
		m.new_line_size.ceil() as usize
	} else {
		font_size + font_size / 5
	};
	let line_widths: Vec<usize> = lines
		.iter()
		.map(|line| {
			let metrics: Vec<_> = line
				.chars()
				.map(|c| font.metrics(c, font_size as f32))
				.collect();
			metrics.iter().map(|m| m.advance_width).sum::<f32>().ceil() as usize
		})
		.collect();
	let total_height = line_height * lines.len();
	let offset_y_base = canvas.margin
		+ canvas
			.height
			.saturating_sub(2 * canvas.margin)
			.saturating_sub(total_height)
			/ 2;

	for (line_idx, (line, line_w)) in lines.iter().zip(line_widths.iter()).enumerate() {
		let offset_x = canvas.margin
			+ canvas
				.width
				.saturating_sub(2 * canvas.margin)
				.saturating_sub(*line_w)
				/ 2;
		let offset_y = offset_y_base + line_idx * line_height;

		let mut cursor_x = offset_x;
		for ch in line.chars() {
			let (raster_metrics, bitmap) = font.rasterize(ch, font_size as f32);
			for ry in 0..raster_metrics.height {
				for rx in 0..raster_metrics.width {
					let pixel = bitmap[ry * raster_metrics.width + rx];
					if pixel > 127 {
						let px = cursor_x + rx;
						let py = offset_y + ry;
						if px < canvas.width && py < canvas.height {
							mask.set(py, px, true);
						}
					}
				}
			}
			let glyph_metrics = font.metrics(ch, font_size as f32);
			cursor_x += glyph_metrics.advance_width.ceil() as usize;
		}
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
		image::imageops::FilterType::Nearest,
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

	#[test]
	fn multiline_uses_fontdue_line_metrics() {
		let font = crate::font::load_default_embedded_font().expect("embedded font should load");
		let canvas = CanvasConfig {
			width: 800,
			height: 400,
			margin: 0,
		};
		let mask = build_shape_mask(&canvas, "DATA\nSCIENCE", &font, 64);
		assert!(
			total_usable_area(&mask) > 0,
			"multi-line shape should produce non-empty mask"
		);
	}
}

#[cfg(test)]
mod image_mask_tests {
	use super::*;

	#[test]
	fn build_image_mask_handles_large_input() {
		use image::{ImageBuffer, Rgba};

		// Build a 1024x1024 mask shaped as a centered disk and feed it through
		// build_image_mask resized down to 800x600. We don't assert wall-clock
		// time here — Nearest is dramatically faster than the previous
		// Lanczos3 path, but timing on CI runners is too noisy for a unit
		// test. Bench coverage in benches/ remains the source of truth for
		// performance changes.
		let mut buf: ImageBuffer<Rgba<u8>, _> = ImageBuffer::new(1024, 1024);
		for (x, y, pixel) in buf.enumerate_pixels_mut() {
			let cx = 512.0_f32;
			let cy = 512.0_f32;
			let dx = x as f32 - cx;
			let dy = y as f32 - cy;
			let inside = (dx * dx + dy * dy).sqrt() < 400.0;
			*pixel = if inside {
				Rgba([0, 0, 0, 255])
			} else {
				Rgba([0, 0, 0, 0])
			};
		}
		let tmp = std::env::temp_dir().join("glyphweave_test_large.png");
		buf.save(&tmp).expect("save large png");

		let canvas = CanvasConfig {
			width: 800,
			height: 600,
			margin: 0,
		};
		let mask = build_image_mask(&canvas, &tmp, 127).expect("build_image_mask");
		assert_eq!(mask.nrows(), 600);
		assert_eq!(mask.ncols(), 800);
		assert!(
			total_usable_area(&mask) > 0,
			"non-trivial mask area expected"
		);

		std::fs::remove_file(&tmp).ok();
	}
}
