use crate::core::model::Rotation;
use crate::mask::calculate_text_size;
use fontdue::Font;
use std::collections::HashMap;
use std::sync::Mutex;

/// Per-`generate()` cache for [`calculate_text_size`].
///
/// The hot path of every layout strategy probes the same
/// `(text, font_size, padding, rotation)` tuples thousands of times while
/// the unique tuple count is typically only a few hundred. Caching those
/// results turns each repeat probe from "iterate every glyph through
/// `font.metrics`" into a single hash-map lookup.
///
/// The cache deliberately does **not** include the font in the key: it is
/// scoped to a single `generate()` invocation (held inside [`LayoutRequest`]),
/// during which the font is constant. Sharing one across runs would risk
/// returning metrics computed against a different font.
#[derive(Debug, Default)]
pub struct TextSizeCache {
	inner: Mutex<HashMap<CacheKey, (usize, usize)>>,
}

type CacheKey = (String, usize, usize, Rotation);

impl TextSizeCache {
	pub fn new() -> Self {
		Self::default()
	}

	/// Look up the cached size for `(text, font_size, padding, rotation)`,
	/// or compute it via [`calculate_text_size`] on a miss and memoize.
	pub fn size_of(
		&self,
		text: &str,
		font: &Font,
		font_size: usize,
		padding: usize,
		rotation: Rotation,
	) -> (usize, usize) {
		let key = (text.to_string(), font_size, padding, rotation);
		if let Some(&hit) = self.inner.lock().unwrap().get(&key) {
			return hit;
		}
		let computed = calculate_text_size(text, font, font_size, padding, rotation);
		self.inner.lock().unwrap().insert(key, computed);
		computed
	}
}
