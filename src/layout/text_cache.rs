use crate::core::model::Rotation;
use crate::mask::calculate_text_size;
use fontdue::Font;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

/// Per-`generate()` cache for [`calculate_text_size`].
///
/// The hot path of every layout strategy probes the same
/// `(text, font_size, padding, rotation)` tuples thousands of times while
/// the unique tuple count is typically only a few hundred. Caching those
/// results turns each repeat probe from "iterate every glyph through
/// `font.metrics`" into a single hash-map lookup.
///
/// The cache is keyed by a precomputed `u64` hash of
/// `(text, font_size, padding, rotation)` so the hit path performs zero
/// allocations: we hash the borrowed `&str` directly, probe the map under
/// a single lock, and verify the full key on hit to defeat (vanishingly
/// rare) hash collisions. The owned `String` is only allocated on a miss,
/// when it must be stored anyway.
///
/// The cache deliberately does **not** include the font in the key: it is
/// scoped to a single `generate()` invocation (held inside [`LayoutRequest`]),
/// during which the font is constant. Sharing one across runs would risk
/// returning metrics computed against a different font.
#[derive(Debug, Default)]
pub struct TextSizeCache {
	inner: Mutex<HashMap<u64, CacheEntry>>,
}

type FullKey = (String, usize, usize, Rotation);
type CacheEntry = (FullKey, (usize, usize));

fn hash_key(text: &str, font_size: usize, padding: usize, rotation: Rotation) -> u64 {
	let mut hasher = DefaultHasher::new();
	text.hash(&mut hasher);
	font_size.hash(&mut hasher);
	padding.hash(&mut hasher);
	rotation.degrees().hash(&mut hasher);
	hasher.finish()
}

fn full_key_matches(
	stored: &FullKey,
	text: &str,
	font_size: usize,
	padding: usize,
	rotation: Rotation,
) -> bool {
	stored.0 == text && stored.1 == font_size && stored.2 == padding && stored.3 == rotation
}

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
		let hash = hash_key(text, font_size, padding, rotation);
		let mut map = self.inner.lock().unwrap();
		if let Some((stored_key, val)) = map.get(&hash)
			&& full_key_matches(stored_key, text, font_size, padding, rotation)
		{
			return *val;
		}
		// Miss (or the astronomically unlikely hash collision): compute and
		// store. On a true collision we overwrite the prior entry; both
		// values are semantically valid for their respective inputs and the
		// cache is per-`generate()`, so any churn is bounded.
		let computed = calculate_text_size(text, font, font_size, padding, rotation);
		map.insert(
			hash,
			((text.to_string(), font_size, padding, rotation), computed),
		);
		computed
	}
}
