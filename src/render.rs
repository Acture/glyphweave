use crate::core::model::{CanvasConfig, CloudPlacement, Rotation};
use crate::mask::calculate_text_size;
use fontdue::Font;
use svg::Document;
use svg::node::element::Text;

pub fn render_svg(
	canvas: &CanvasConfig,
	placements: &[CloudPlacement],
	font: &Font,
	font_family: &str,
) -> String {
	let mut doc = Document::new()
		.set("width", canvas.width)
		.set("height", canvas.height)
		.set("viewBox", (0, 0, canvas.width, canvas.height))
		.set("xmlns", "http://www.w3.org/2000/svg")
		.set("xmlns:xlink", "http://www.w3.org/1999/xlink");

	for placement in placements {
		let element = match placement.rotation {
			Rotation::Deg0 => Text::new(&placement.word)
				.set("x", placement.x)
				.set("y", placement.y)
				.set("font-family", font_family)
				.set("font-size", placement.font_size)
				.set("fill", placement.color.as_str())
				.set("dominant-baseline", "hanging")
				.set("text-anchor", "start"),
			Rotation::Deg90 => {
				// Unrotated bbox in local coords is (0,0)-(w,h). After
				// rotate(90) (clockwise) around origin, it becomes
				// (-h,0)-(0,w). Translating by (placement.x + h, placement.y)
				// puts the rotated bbox's top-left at the reserved
				// (placement.x, placement.y), matching the mask reservation
				// produced by `calculate_text_size` with rotation.
				let (_unrotated_w, unrotated_h) = calculate_text_size(
					&placement.word,
					font,
					placement.font_size,
					0,
					Rotation::Deg0,
				);
				let tx = placement.x + unrotated_h;
				let ty = placement.y;
				Text::new(&placement.word)
					.set("x", 0)
					.set("y", 0)
					.set("font-family", font_family)
					.set("font-size", placement.font_size)
					.set("fill", placement.color.as_str())
					.set("dominant-baseline", "hanging")
					.set("text-anchor", "start")
					.set("transform", format!("translate({tx} {ty}) rotate(90)"))
			}
		};

		doc = doc.add(element);
	}

	doc.to_string()
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
		let svg = render_svg(&canvas, &placements, &font, "Test");
		let expected = format!("translate({} {}) rotate(90)", 50 + h, 60);
		assert!(
			svg.contains(&expected),
			"expected svg to contain `{expected}`, got: {svg}"
		);
		assert!(!svg.contains("rotate(90 "));
	}
}
