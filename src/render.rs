use crate::core::model::{CanvasConfig, CloudPlacement, RenderMetadata, Rotation};
use crate::mask::calculate_text_size;
use fontdue::Font;
use svg::Document;
use svg::Node;
use svg::node::element::{Description, Element, Text};

pub fn render_svg(
	canvas: &CanvasConfig,
	placements: &[CloudPlacement],
	font: &Font,
	font_family: &str,
	metadata: &RenderMetadata,
) -> String {
	let mut doc = Document::new()
		.set("width", canvas.width)
		.set("height", canvas.height)
		.set("viewBox", (0, 0, canvas.width, canvas.height))
		.set("xmlns", "http://www.w3.org/2000/svg")
		.set("xmlns:xlink", "http://www.w3.org/1999/xlink");

	let desc = Description::new().add(svg::node::Text::new(format!(
		"GlyphWeave word cloud — seed {} — algorithm {} — {} words — {:.1}% fill",
		metadata.seed,
		metadata.algorithm,
		metadata.placed_words,
		metadata.fill_ratio * 100.0,
	)));
	doc = doc.add(desc);

	let mut gw = Element::new("glyphweave");
	gw.get_attributes_mut().insert(
		"xmlns".into(),
		"https://github.com/Acture/glyphweave/v1".into(),
	);
	gw.get_attributes_mut()
		.insert("seed".into(), metadata.seed.to_string().into());
	gw.get_attributes_mut()
		.insert("algorithm".into(), metadata.algorithm.into());
	gw.get_attributes_mut().insert(
		"placed_words".into(),
		metadata.placed_words.to_string().into(),
	);
	gw.get_attributes_mut().insert(
		"fill_ratio".into(),
		format!("{:.4}", metadata.fill_ratio).into(),
	);
	let mut metadata_node = Element::new("metadata");
	metadata_node.append(gw);
	doc = doc.add(metadata_node);

	for placement in placements {
		let deg = placement.rotation.degrees();
		let element = if deg == 0 {
			Text::new(&placement.word)
				.set("x", placement.x)
				.set("y", placement.y)
				.set("font-family", font_family)
				.set("font-size", placement.font_size)
				.set("fill", placement.color.as_str())
				.set("dominant-baseline", "text-before-edge")
				.set("text-anchor", "start")
		} else {
			// Unrotated text bbox is (0,0)-(uw,uh). Rotating by `deg` around
			// the origin yields a new AABB; we translate so that AABB's
			// top-left aligns with placement.(x, y) — the mask reservation
			// produced by calculate_text_size with the same rotation.
			let (unrotated_w, unrotated_h) = calculate_text_size(
				&placement.word,
				font,
				placement.font_size,
				0,
				Rotation::Deg0,
			);
			let uw = unrotated_w as f32;
			let uh = unrotated_h as f32;
			let radians = placement.rotation.radians();
			let (cos, sin) = (radians.cos(), radians.sin());
			let corners = [(0.0, 0.0), (uw, 0.0), (0.0, uh), (uw, uh)];
			let rotated = corners.map(|(x, y)| (x * cos - y * sin, x * sin + y * cos));
			let min_x = rotated.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
			let min_y = rotated.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
			let tx = placement.x as f32 - min_x;
			let ty = placement.y as f32 - min_y;
			Text::new(&placement.word)
				.set("x", 0)
				.set("y", 0)
				.set("font-family", font_family)
				.set("font-size", placement.font_size)
				.set("fill", placement.color.as_str())
				.set("dominant-baseline", "text-before-edge")
				.set("text-anchor", "start")
				.set("transform", format!("translate({tx} {ty}) rotate({deg})"))
		};

		doc = doc.add(element);
	}

	doc.to_string()
}

#[cfg(test)]
mod baseline_tests {
	use super::*;
	use crate::core::model::{CanvasConfig, CloudPlacement, Rotation};

	#[test]
	fn render_uses_text_before_edge_baseline() {
		let canvas = CanvasConfig {
			width: 100,
			height: 50,
			margin: 0,
		};
		let placements = vec![CloudPlacement {
			word: "x".into(),
			x: 10,
			y: 20,
			font_size: 12,
			color: "#000".into(),
			rotation: Rotation::Deg0,
		}];
		let font = crate::font::load_font_from_file(
			std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/NotoSansSC-Regular.ttf"),
		)
		.expect("test font should load");
		let metadata = RenderMetadata {
			seed: 0,
			placed_words: 1,
			fill_ratio: 0.0,
			algorithm: "test",
		};
		let svg = render_svg(&canvas, &placements, &font, "Test Font", &metadata);
		assert!(
			svg.contains("text-before-edge"),
			"must use text-before-edge baseline"
		);
		assert!(svg.contains(r#"x="10""#));
		assert!(svg.contains(r#"y="20""#));
	}
}

#[cfg(all(test, feature = "embedded_fonts"))]
mod tests {
	use super::*;
	use crate::core::model::CloudPlacement;

	#[test]
	fn deg90_uses_translate_then_rotate() {
		let font = crate::font::load_default_embedded_font().expect("embedded font");
		let canvas = CanvasConfig {
			width: 400,
			height: 300,
			margin: 0,
		};
		let font_size = 24;
		let (_w, h) = calculate_text_size("Hello", &font, font_size, 0, Rotation::Deg0);
		let placements = vec![CloudPlacement {
			word: "Hello".to_string(),
			x: 50,
			y: 60,
			font_size,
			color: "#111111".to_string(),
			rotation: Rotation::Deg90,
		}];
		let metadata = RenderMetadata {
			seed: 0,
			placed_words: 1,
			fill_ratio: 0.0,
			algorithm: "test",
		};
		let svg = render_svg(&canvas, &placements, &font, "Test", &metadata);
		let expected = format!("translate({} {}) rotate(90)", 50 + h, 60);
		assert!(
			svg.contains(&expected),
			"expected svg to contain `{expected}`, got: {svg}"
		);
		assert!(!svg.contains("rotate(90 "));
	}
}
