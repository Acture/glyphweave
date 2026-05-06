use crate::core::model::{CloudPlacement, Rotation, StyleConfig, WordEntry};
use crate::layout::LayoutRequest;
use crate::layout::bitmask::BitMask;
use crate::layout::text_cache::TextSizeCache;
use fontdue::Font;
use indicatif::{ProgressBar, ProgressStyle};
use ndarray::Array2;
use rand::RngCore;

// # Per-strategy tuning constants
//
// Each layout strategy maintains its own tuning constants. Values
// differ intentionally based on internal cost characteristics; they
// are documented here for cross-comparison.
//
// | Constant                  | fast_grid | mcts | random_baseline | sa  | spiral_greedy |
// |---------------------------|-----------|------|-----------------|-----|---------------|
// | POOL_REFILL_THRESHOLD     | 512       | 256  | 256             | 256 | n/a           |
// | CANDIDATE_TRIALS          | 48        | 64   | 24              | 48  | n/a           |
// | INTEGRAL_REBUILD_INTERVAL | 64        | n/a  | n/a             | n/a | n/a           |
//
// To re-tune all strategies coherently, update each module's const
// and the snapshot fixtures in `tests/golden/`.

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

pub fn total_area(mask: &BitMask) -> usize {
	mask.count_ones()
}

pub fn available_positions(mask: &BitMask) -> Vec<(usize, usize)> {
	mask.indexed_set().collect()
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

	pub fn new(mask: &BitMask) -> Self {
		Self::with_interval(mask, Self::DEFAULT_REBUILD_INTERVAL)
	}

	pub fn with_interval(mask: &BitMask, rebuild_interval: usize) -> Self {
		Self {
			integral: build_integral(mask),
			pending_rects: Vec::new(),
			rebuild_interval: rebuild_interval.max(1),
		}
	}

	/// O(1) availability check. Returns `true` iff every cell of `rect` is
	/// inside the mask, currently free, and does not overlap any rectangle
	/// committed since the last rebuild.
	pub fn is_available(&self, mask: &BitMask, rect: Rect) -> bool {
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
	pub fn commit_rect(&mut self, mask: &BitMask, rect: Rect) {
		self.pending_rects.push(rect);
		if self.pending_rects.len() >= self.rebuild_interval {
			self.integral = build_integral(mask);
			self.pending_rects.clear();
		}
	}

	/// Capture the current pending-rect log length so a scoped batch of
	/// commits can later be unwound via [`IncrementalAvailability::restore`].
	pub fn snapshot(&self) -> AvailabilitySnapshot {
		AvailabilitySnapshot {
			pending_count: self.pending_rects.len(),
		}
	}

	/// Roll the availability state back to a previous [`snapshot`]. The caller
	/// must have already restored `mask` to the cell pattern it had at the
	/// time the snapshot was taken (typically by re-setting the cells of the
	/// rectangles committed in the meantime). The integral image is rebuilt
	/// from `mask` so subsequent queries observe the rolled-back state.
	///
	/// Note: if any `commit_rect` between snapshot and restore triggered an
	/// internal rebuild (i.e. `pending_rects` was cleared), a `truncate`
	/// based on `snap.pending_count` would silently leak the rollout-only
	/// rects committed after the rebuild (truncate is a no-op when the
	/// vector is already shorter than the target length). The caller has
	/// already restored `mask` to the snapshot-time cell pattern, so the
	/// rebuilt integral image fully encodes the rolled-back state and an
	/// empty pending list is sufficient *and* correct.
	pub fn restore(&mut self, mask: &BitMask, _snap: AvailabilitySnapshot) {
		self.pending_rects.clear();
		self.integral = build_integral(mask);
	}

	#[cfg(test)]
	pub(crate) fn pending_count(&self) -> usize {
		self.pending_rects.len()
	}
}

#[derive(Debug, Clone, Copy)]
pub struct AvailabilitySnapshot {
	/// Retained as an opaque token for API stability and future extension
	/// (e.g. recording additional rollback state). The current `restore`
	/// implementation rebuilds the integral image and clears the pending
	/// list unconditionally, so this field is intentionally unused.
	#[allow(dead_code)]
	pending_count: usize,
}

fn build_integral(mask: &BitMask) -> Array2<u32> {
	let rows = mask.nrows();
	let cols = mask.ncols();
	let mut integral = Array2::<u32>::zeros((rows + 1, cols + 1));

	for y in 0..rows {
		for x in 0..cols {
			let value = if mask.get(y, x) { 1 } else { 0 };
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

pub fn occupy_area(mask: &mut BitMask, rect: Rect) -> usize {
	mask.clear_rect(rect.x, rect.y, rect.w, rect.h)
}

pub fn random_index(rng: &mut dyn RngCore, len: usize) -> usize {
	if len <= 1 {
		return 0;
	}
	// Rejection sampling removes modulo bias: accept only draws that fall in
	// the largest multiple of `len` that fits in u64.
	let len_u64 = len as u64;
	// rem = 2^64 mod len_u64 (computed safely without overflow).
	let rem = (u64::MAX % len_u64).wrapping_add(1) % len_u64;
	// Largest accepted draw is `K*len - 1` where K = floor(2^64 / len). When
	// `rem == 0`, len divides 2^64 and every draw is accepted.
	let threshold = 0u64.wrapping_sub(rem);
	loop {
		let r = rng.next_u64();
		if rem == 0 || r < threshold {
			return (r % len_u64) as usize;
		}
	}
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
		if cursor < 0.0 {
			return Some(word);
		}
	}

	words.last()
}

pub fn pick_color<'a>(colors: &'a [String], rng: &mut dyn RngCore) -> &'a str {
	debug_assert!(!colors.is_empty(), "pick_color called with empty palette");
	if colors.is_empty() {
		return "";
	}
	let idx = random_index(rng, colors.len());
	colors[idx].as_str()
}

pub fn random_unit_f32(rng: &mut dyn RngCore) -> f32 {
	// Use the high 24 bits of the draw to fill a single-precision mantissa
	// exactly once, matching the standard `rand` convention for [0, 1) f32
	// generation.
	const SCALE: f32 = 1.0 / ((1u32 << 24) as f32);
	((rng.next_u64() >> 40) as f32) * SCALE
}

pub fn descending_font_sizes(style: &StyleConfig) -> impl Iterator<Item = usize> {
	(*style.font_size_range.start()..=*style.font_size_range.end()).rev()
}

#[allow(clippy::too_many_arguments)]
pub fn find_fit_at_position(
	availability: &IncrementalAvailability,
	mask: &BitMask,
	x: usize,
	y: usize,
	word: &str,
	style: &StyleConfig,
	font: &Font,
	cache: &TextSizeCache,
	evaluations: &mut usize,
) -> Option<(usize, Rotation, Rect)> {
	for size in descending_font_sizes(style) {
		for rotation in &style.rotations {
			let (w, h) = cache.size_of(word, font, size, style.padding, *rotation);
			let rect = Rect { x, y, w, h };
			*evaluations += 1;
			if availability.is_available(mask, rect) {
				return Some((size, *rotation, rect));
			}
		}
	}

	None
}

#[allow(clippy::too_many_arguments)]
pub fn sample_candidate(
	mask: &BitMask,
	availability: &IncrementalAvailability,
	positions: &mut Vec<(usize, usize)>,
	request: &LayoutRequest<'_>,
	rng: &mut dyn RngCore,
	max_trials: usize,
	evaluations: &mut usize,
) -> Option<PlacementCandidate> {
	for _ in 0..max_trials {
		if positions.is_empty() {
			return None;
		}

		let idx = random_index(rng, positions.len());
		let (y, x) = positions[idx];
		if !mask.get(y, x) {
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
			&request.text_size_cache,
			evaluations,
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
	mask: &mut BitMask,
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

#[cfg(test)]
mod restore_leak_test {
	use super::*;
	use crate::layout::bitmask::BitMask;

	#[test]
	fn restore_clears_pending_rects_across_internal_rebuild() {
		// Reproduce R1 trace: parent commits 60 rects, child rollout commits 7
		// spanning an internal rebuild (commit_rect threshold = 64).
		let mask = BitMask::from_fn(200, 200, |_, _| true);
		let mut avail = IncrementalAvailability::new(&mask);

		// Phase 1: parent commits 60 rects. We deliberately do not mutate the
		// mask: the test exercises the pending_rects bookkeeping path in
		// isolation. is_available semantics are covered elsewhere.
		for i in 0..60 {
			let r = Rect {
				x: (i * 3) % 198,
				y: (i / 60) * 3,
				w: 2,
				h: 2,
			};
			avail.commit_rect(&mask, r);
		}
		assert_eq!(avail.pending_count(), 60);

		// Phase 2: snapshot at parent state.
		let snap = avail.snapshot();

		// Phase 3: rollout commits 7 rects. The 4th push hits the rebuild
		// threshold (>= 64), which clears pending_rects; the trailing 3 land
		// in a freshly-cleared list.
		for i in 0..7 {
			let r = Rect {
				x: 100 + i * 3,
				y: 50,
				w: 2,
				h: 2,
			};
			avail.commit_rect(&mask, r);
		}
		assert!(
			avail.pending_count() < 60,
			"rebuild should have fired during rollout commits"
		);

		// Phase 4: restore. Without the fix, truncate(snap.pending_count=60)
		// is a no-op on a 3-element vec -> 3 stale rollout rects leak into
		// the parent's pending list and silently reject overlapping cells.
		avail.restore(&mask, snap);

		assert_eq!(
			avail.pending_count(),
			0,
			"restore must clear pending_rects entirely (R1 P0 regression test)"
		);
	}
}
