use crate::cli::args::PaletteKind;
use glyphweave::core::error::GlyphWeaveError;

pub fn resolve_colors(
	explicit_colors: Option<Vec<String>>,
	palette_kind: PaletteKind,
	palette_base: &str,
	palette_size: usize,
) -> Result<Vec<String>, GlyphWeaveError> {
	if let Some(colors) = explicit_colors {
		if colors.is_empty() {
			return Err(GlyphWeaveError::InvalidConfig(
				"--colors provided but empty".to_string(),
			));
		}
		return Ok(colors);
	}

	generate_palette(palette_kind, palette_base, palette_size)
}

pub fn generate_palette(
	kind: PaletteKind,
	base_hex: &str,
	size: usize,
) -> Result<Vec<String>, GlyphWeaveError> {
	let size = size.max(1);

	match kind {
		PaletteKind::Pastel => Ok(repeat_palette(
			&[
				"#B8E1FF", "#FFD6E0", "#FFF3B0", "#CDEAC0", "#E4C1F9", "#FEE1C7",
			],
			size,
		)),
		PaletteKind::Earth => Ok(repeat_palette(
			&[
				"#3E2723", "#6D4C41", "#8D6E63", "#A1887F", "#5D4037", "#4E342E",
			],
			size,
		)),
		PaletteKind::Vibrant => Ok(repeat_palette(
			&[
				"#FF3B30", "#FF9500", "#FFCC00", "#34C759", "#007AFF", "#AF52DE",
			],
			size,
		)),
		PaletteKind::Auto
		| PaletteKind::Complementary
		| PaletteKind::Triadic
		| PaletteKind::Analogous
		| PaletteKind::Monochrome => generate_hsl_palette(kind, base_hex, size),
	}
}

fn generate_hsl_palette(
	kind: PaletteKind,
	base_hex: &str,
	size: usize,
) -> Result<Vec<String>, GlyphWeaveError> {
	let (r, g, b) = parse_hex_color(base_hex)?;
	let (h, s, l) = rgb_to_hsl(r, g, b);
	let mut out = Vec::with_capacity(size);

	for i in 0..size {
		let t = if size == 1 {
			0.5
		} else {
			i as f32 / (size - 1) as f32
		};

		let (hh, ss, ll) = match kind {
			PaletteKind::Auto => {
				let hues = [h - 30.0, h, h + 30.0, h + 160.0, h + 200.0, h + 240.0];
				let hue = hues[i % hues.len()];
				let light = 0.35 + 0.35 * t;
				(hue, (s * 0.85).clamp(0.35, 0.85), light)
			}
			PaletteKind::Complementary => {
				let hue = if i % 2 == 0 { h } else { h + 180.0 };
				let light = if i % 2 == 0 {
					0.35 + 0.25 * t
				} else {
					0.45 + 0.25 * t
				};
				(hue, (s * 0.9).clamp(0.4, 0.9), light)
			}
			PaletteKind::Triadic => {
				let hue = h + 120.0 * (i % 3) as f32;
				let light = 0.33 + 0.35 * t;
				(hue, (s * 0.88).clamp(0.4, 0.9), light)
			}
			PaletteKind::Analogous => {
				let spread = 60.0;
				let hue = h + spread * (t - 0.5);
				(
					hue,
					(s * 0.85).clamp(0.35, 0.85),
					(l * 0.65 + 0.15 + 0.3 * t).clamp(0.2, 0.82),
				)
			}
			PaletteKind::Monochrome => (h, (s * 0.75).clamp(0.15, 0.75), 0.2 + 0.6 * t),
			PaletteKind::Pastel | PaletteKind::Earth | PaletteKind::Vibrant => {
				debug_assert!(false, "preset palettes handled by generate_palette");
				(h, s, l)
			}
		};

		let (rr, gg, bb) = hsl_to_rgb(hh, ss, ll);
		out.push(format!("#{:02X}{:02X}{:02X}", rr, gg, bb));
	}

	Ok(out)
}

fn repeat_palette(colors: &[&str], size: usize) -> Vec<String> {
	let mut out = Vec::with_capacity(size);
	for i in 0..size {
		out.push(colors[i % colors.len()].to_string());
	}
	out
}

fn parse_hex_color(input: &str) -> Result<(u8, u8, u8), GlyphWeaveError> {
	let text = input.trim();
	let clean = text.strip_prefix('#').unwrap_or(text);

	if clean.len() != 6 {
		return Err(GlyphWeaveError::InvalidConfig(format!(
			"invalid palette base color '{input}', expected #RRGGBB"
		)));
	}

	let r = u8::from_str_radix(&clean[0..2], 16).map_err(|_| {
		GlyphWeaveError::InvalidConfig(format!(
			"invalid palette base color '{input}', expected #RRGGBB"
		))
	})?;
	let g = u8::from_str_radix(&clean[2..4], 16).map_err(|_| {
		GlyphWeaveError::InvalidConfig(format!(
			"invalid palette base color '{input}', expected #RRGGBB"
		))
	})?;
	let b = u8::from_str_radix(&clean[4..6], 16).map_err(|_| {
		GlyphWeaveError::InvalidConfig(format!(
			"invalid palette base color '{input}', expected #RRGGBB"
		))
	})?;

	Ok((r, g, b))
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
	let r = r as f32 / 255.0;
	let g = g as f32 / 255.0;
	let b = b as f32 / 255.0;

	let max = r.max(g.max(b));
	let min = r.min(g.min(b));
	let delta = max - min;

	let l = (max + min) / 2.0;

	if delta.abs() < f32::EPSILON {
		return (0.0, 0.0, l);
	}

	let s = delta / (1.0 - (2.0 * l - 1.0).abs());

	let h = if (max - r).abs() < f32::EPSILON {
		60.0 * (((g - b) / delta) % 6.0)
	} else if (max - g).abs() < f32::EPSILON {
		60.0 * (((b - r) / delta) + 2.0)
	} else {
		60.0 * (((r - g) / delta) + 4.0)
	};

	(normalize_hue(h), s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
	let h = normalize_hue(h);

	let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
	let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
	let m = l - c / 2.0;

	let (r1, g1, b1) = if h < 60.0 {
		(c, x, 0.0)
	} else if h < 120.0 {
		(x, c, 0.0)
	} else if h < 180.0 {
		(0.0, c, x)
	} else if h < 240.0 {
		(0.0, x, c)
	} else if h < 300.0 {
		(x, 0.0, c)
	} else {
		(c, 0.0, x)
	};

	let r = ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
	let g = ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
	let b = ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;

	(r, g, b)
}

fn normalize_hue(h: f32) -> f32 {
	let mut hue = h % 360.0;
	if hue < 0.0 {
		hue += 360.0;
	}
	hue
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn complementary_palette_has_expected_size() {
		let colors = generate_palette(PaletteKind::Complementary, "#3366CC", 6)
			.expect("palette should be generated");
		assert_eq!(colors.len(), 6);
		assert!(colors.iter().all(|c| c.starts_with('#') && c.len() == 7));
	}
}
