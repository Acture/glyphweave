use crate::core::model::{CloudPlacement, Rotation, StyleConfig, WordEntry};
use crate::layout::LayoutRequest;
use crate::mask::calculate_text_size;
use fontdue::Font;
use indicatif::{ProgressBar, ProgressStyle};
use ndarray::Array2;
use rand::RngCore;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
	pub x: usize,
	pub y: usize,
	pub w: usize,
	pub h: usize,
}

#[derive(Debug, Clone)]
pub struct PlacementCandidate {
	pub word: String,
	pub word_weight: f32,
	pub rect: Rect,
	pub font_size: usize,
	pub rotation: Rotation,
}

impl Rect {
	pub fn area(self) -> usize {
		self.w * self.h
	}
}

pub fn total_area(mask: &Array2<bool>) -> usize {
	mask.iter().filter(|&&v| v).count()
}

pub fn available_positions(mask: &Array2<bool>) -> Vec<(usize, usize)> {
	mask.indexed_iter()
		.filter_map(|((y, x), value)| if *value { Some((y, x)) } else { None })
		.collect()
}

/// Maintains an integral image of the available mask plus a small log of
/// recently committed rectangles, so rectangle-availability checks become
/// O(1) amortized for every layout strategy.
///
/// ## Usage contract
///
/// 1. Build once per `mask` with [`IncrementalAvailability::new`].
/// 2. Probe rectangles with [`IncrementalAvailability::is_available`] before
///    placing them. The check considers both the underlying mask (via the
///    integral image) and any rects already committed since the last rebuild.
/// 3. After placing a rectangle, the caller MUST update the mask first
///    (typically via [`occupy_area`]) and THEN call
///    [`IncrementalAvailability::commit_rect`]. The pending list is what
///    keeps subsequent O(1) queries correct in the window between rebuilds:
///    the integral image still reports those cells as available until the
///    next rebuild, so the pending overlap test compensates.
/// 4. Once the pending list reaches `rebuild_interval` entries, the integral
///    is rebuilt from the (already updated) mask and the pending list cleared.
pub struct IncrementalAvailability {
	integral: Array2<u32>,
	pending_rects: Vec<Rect>,
	rebuild_interval: usize,
}

impl IncrementalAvailability {
	const DEFAULT_REBUILD_INTERVAL: usize = 64;

	pub fn new(mask: &Array2<bool>) -> Self {
		Self::with_interval(mask, Self::DEFAULT_REBUILD_INTERVAL)
	}

	pub fn with_interval(mask: &Array2<bool>, rebuild_interval: usize) -> Self {
		Self {
			integral: build_integral(mask),
			pending_rects: Vec::new(),
			rebuild_interval: rebuild_interval.max(1),
		}
	}

	/// O(1) availability check. Returns `true` iff every cell of `rect` is
	/// inside the mask, currently free, and does not overlap any rectangle
	/// committed since the last rebuild.
	pub fn is_available(&self, mask: &Array2<bool>, rect: Rect) -> bool {
		if rect.w == 0 || rect.h == 0 {
			return false;
		}
		if rect.x + rect.w > mask.ncols() || rect.y + rect.h > mask.nrows() {
			return false;
		}
		let area = rect.area() as u32;
		if rect_sum(&self.integral, rect) != area {
			return false;
		}
		!self
			.pending_rects
			.iter()
			.any(|pending| intersects(*pending, rect))
	}

	/// Record a rectangle that has just been committed to `mask`. The caller
	/// must already have cleared the cells of `rect` in `mask` (see
	/// [`occupy_area`]). When the pending log fills up the integral image is
	/// rebuilt from `mask` so subsequent queries stay O(1) without an
	/// ever-growing overlap test.
	pub fn commit_rect(&mut self, mask: &Array2<bool>, rect: Rect) {
		self.pending_rects.push(rect);
		if self.pending_rects.len() >= self.rebuild_interval {
			self.integral = build_integral(mask);
			self.pending_rects.clear();
		}
	}
}

fn build_integral(mask: &Array2<bool>) -> Array2<u32> {
	let rows = mask.nrows();
	let cols = mask.ncols();
	let mut integral = Array2::<u32>::zeros((rows + 1, cols + 1));

	for y in 0..rows {
		for x in 0..cols {
			let value = if mask[[y, x]] { 1 } else { 0 };
			integral[[y + 1, x + 1]] =
				value + integral[[y, x + 1]] + integral[[y + 1, x]] - integral[[y, x]];
		}
	}

	integral
}

fn rect_sum(integral: &Array2<u32>, rect: Rect) -> u32 {
	let x1 = rect.x;
	let y1 = rect.y;
	let x2 = rect.x + rect.w;
	let y2 = rect.y + rect.h;

	integral[[y2, x2]] + integral[[y1, x1]] - integral[[y1, x2]] - integral[[y2, x1]]
}

pub fn occupy_area(mask: &mut Array2<bool>, rect: Rect) -> usize {
	let mut consumed = 0usize;

	for dy in 0..rect.h {
		for dx in 0..rect.w {
			let cell = &mut mask[[rect.y + dy, rect.x + dx]];
			if *cell {
				*cell = false;
				consumed += 1;
			}
		}
	}

	consumed
}

pub fn random_index(rng: &mut dyn RngCore, len: usize) -> usize {
	if len <= 1 {
		return 0;
	}
	(rng.next_u64() as usize) % len
}

pub fn pick_weighted_word<'a>(
	words: &'a [WordEntry],
	rng: &mut dyn RngCore,
) -> Option<&'a WordEntry> {
	if words.is_empty() {
		return None;
	}

	let total_weight = words
		.iter()
		.map(|w| w.weight.max(0.0) as f64)
		.fold(0.0f64, |acc, w| acc + w);

	if total_weight <= f64::EPSILON {
		return words.get(random_index(rng, words.len()));
	}

	let rand_unit = (rng.next_u64() as f64) / (u64::MAX as f64);
	let mut cursor = rand_unit * total_weight;

	for word in words {
		cursor -= word.weight.max(0.0) as f64;
		if cursor <= 0.0 {
			return Some(word);
		}
	}

	words.last()
}

pub fn pick_color<'a>(colors: &'a [String], rng: &mut dyn RngCore) -> &'a str {
	let idx = random_index(rng, colors.len());
	colors[idx].as_str()
}

pub fn random_unit_f32(rng: &mut dyn RngCore) -> f32 {
	(rng.next_u64() as f64 / u64::MAX as f64) as f32
}

pub fn descending_font_sizes(style: &StyleConfig) -> impl Iterator<Item = usize> {
	(*style.font_size_range.start()..=*style.font_size_range.end()).rev()
}

pub fn find_fit_at_position(
	availability: &IncrementalAvailability,
	mask: &Array2<bool>,
	x: usize,
	y: usize,
	word: &str,
	style: &StyleConfig,
	font: &Font,
) -> Option<(usize, Rotation, Rect)> {
	for size in descending_font_sizes(style) {
		for rotation in &style.rotations {
			let (w, h) = calculate_text_size(word, font, size, style.padding, *rotation);
			let rect = Rect { x, y, w, h };
			if availability.is_available(mask, rect) {
				return Some((size, *rotation, rect));
			}
		}
	}

	None
}

pub fn sample_candidate(
	mask: &Array2<bool>,
	availability: &IncrementalAvailability,
	positions: &mut Vec<(usize, usize)>,
	request: &LayoutRequest<'_>,
	rng: &mut dyn RngCore,
	max_trials: usize,
) -> Option<PlacementCandidate> {
	for _ in 0..max_trials {
		if positions.is_empty() {
			return None;
		}

		let idx = random_index(rng, positions.len());
		let (y, x) = positions[idx];
		if !mask[[y, x]] {
			positions.swap_remove(idx);
			continue;
		}

		let word = pick_weighted_word(request.words, rng)?;
		if let Some((font_size, rotation, rect)) = find_fit_at_position(
			availability,
			mask,
			x,
			y,
			&word.text,
			request.style,
			request.font,
		) {
			return Some(PlacementCandidate {
				word: word.text.clone(),
				word_weight: word.weight.max(0.0),
				rect,
				font_size,
				rotation,
			});
		}
	}

	None
}

pub fn apply_candidate(
	mask: &mut Array2<bool>,
	candidate: &PlacementCandidate,
	color: &str,
) -> (CloudPlacement, usize) {
	let consumed = occupy_area(mask, candidate.rect);
	let placed = placement(
		&candidate.word,
		candidate.rect,
		candidate.font_size,
		color,
		candidate.rotation,
	);
	(placed, consumed)
}

pub fn candidate_quality(candidate: &PlacementCandidate, total_usable_area: usize) -> f32 {
	if total_usable_area == 0 {
		return 0.0;
	}
	let area_score = candidate.rect.area() as f32 / total_usable_area as f32;
	area_score + candidate.word_weight * 0.01
}

pub fn placement(
	word: &str,
	rect: Rect,
	font_size: usize,
	color: &str,
	rotation: Rotation,
) -> CloudPlacement {
	CloudPlacement {
		word: word.to_string(),
		x: rect.x,
		y: rect.y,
		font_size,
		color: color.to_string(),
		rotation,
	}
}

pub fn create_progress_bar(show_progress: bool) -> Option<ProgressBar> {
	if !show_progress {
		return None;
	}

	let pb = ProgressBar::new(100);
	pb.set_style(
		ProgressStyle::with_template("[{bar:40.cyan/blue}] {pos:>3}%")
			.expect("progress style template should be valid")
			.progress_chars("=>-"),
	);
	Some(pb)
}

pub fn update_progress(pb: &Option<ProgressBar>, percent: usize) {
	if let Some(progress) = pb {
		progress.set_position(percent.min(100) as u64);
	}
}

pub fn finish_progress(pb: &Option<ProgressBar>) {
	if let Some(progress) = pb {
		progress.finish_and_clear();
	}
}

pub fn intersects(a: Rect, b: Rect) -> bool {
	a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}
