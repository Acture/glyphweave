use crate::core::error::GlyphWeaveError;
use crate::layout::common::{
	IncrementalAvailability, Rect, create_progress_bar, descending_font_sizes, finish_progress,
	occupy_area, pick_color, pick_weighted_word, placement, total_area, update_progress,
};
use crate::layout::{LayoutRequest, LayoutResult, LayoutStrategy};
use crate::mask::mask_centroid;
use rand::RngCore;
use std::sync::{Arc, OnceLock};

const SEARCH_RADIUS_LIMIT: usize = 220;

static SPIRAL_OFFSETS: OnceLock<Arc<[(isize, isize)]>> = OnceLock::new();

fn cached_spiral_offsets() -> &'static [(isize, isize)] {
	SPIRAL_OFFSETS
		.get_or_init(|| Arc::from(spiral_offsets(SEARCH_RADIUS_LIMIT)))
		.as_ref()
}

pub struct SpiralGreedyStrategy;

impl LayoutStrategy for SpiralGreedyStrategy {
	fn place(
		&self,
		request: &LayoutRequest<'_>,
		rng: &mut dyn RngCore,
	) -> Result<LayoutResult, GlyphWeaveError> {
		let mut mask = request.mask.clone();
		let total_usable_area = total_area(&mask);
		if total_usable_area == 0 {
			return Err(GlyphWeaveError::Generation(
				"shape mask has no usable area".to_string(),
			));
		}

		let mut center = mask_centroid(&mask);
		let offsets = cached_spiral_offsets();
		let mut availability = IncrementalAvailability::new(&mask);
		let mut placements = Vec::new();
		let mut attempts = 0usize;
		let mut used_area = 0usize;
		let progress = create_progress_bar(request.show_progress);

		while attempts < request.max_try_count {
			let fill_ratio = used_area as f32 / total_usable_area as f32;
			if fill_ratio >= request.ratio_threshold {
				break;
			}

			attempts += 1;
			let Some(word_entry) = pick_weighted_word(request.words, rng) else {
				break;
			};

			let mut placed = None;
			'font_search: for size in descending_font_sizes(request.style) {
				for rotation in &request.style.rotations {
					// calculate_text_size is hoisted out of the offset loop so it runs
					// once per (size, rotation) instead of once per spiral cell.
					let (w, h) = request.text_size_cache.size_of(
						&word_entry.text,
						request.font,
						size,
						request.style.padding,
						*rotation,
					);
					for &(dy, dx) in offsets {
						let x = center.0 as isize + dx;
						let y = center.1 as isize + dy;

						if x < 0 || y < 0 {
							continue;
						}

						let rect = Rect {
							x: x as usize,
							y: y as usize,
							w,
							h,
						};

						if availability.is_available(&mask, rect) {
							placed = Some((size, *rotation, rect));
							break 'font_search;
						}
					}
				}
			}

			if let Some((font_size, rotation, rect)) = placed {
				used_area += occupy_area(&mut mask, rect);
				availability.commit_rect(&mask, rect);
				let color = pick_color(&request.style.colors, rng);
				placements.push(placement(
					&word_entry.text,
					rect,
					font_size,
					color,
					rotation,
				));

				center = (rect.x + rect.w / 2, rect.y + rect.h / 2);
				if !mask[[
					center.1.min(mask.nrows() - 1),
					center.0.min(mask.ncols() - 1),
				]] {
					center = mask_centroid(&mask);
				}
			}

			let ratio_progress = (used_area * 100) / total_usable_area;
			let try_progress = (attempts * 100) / request.max_try_count;
			update_progress(&progress, ratio_progress.max(try_progress));
		}

		finish_progress(&progress);

		Ok(LayoutResult {
			placements,
			attempts,
			used_area,
		})
	}
}

fn spiral_offsets(radius_limit: usize) -> Vec<(isize, isize)> {
	let mut offsets = Vec::with_capacity((2 * radius_limit + 1).pow(2));
	offsets.push((0, 0));

	let mut x = 0isize;
	let mut y = 0isize;
	let mut step = 1isize;

	while x.unsigned_abs() <= radius_limit && y.unsigned_abs() <= radius_limit {
		for _ in 0..step {
			x += 1;
			offsets.push((y, x));
		}
		for _ in 0..step {
			y += 1;
			offsets.push((y, x));
		}
		step += 1;

		for _ in 0..step {
			x -= 1;
			offsets.push((y, x));
		}
		for _ in 0..step {
			y -= 1;
			offsets.push((y, x));
		}
		step += 1;

		if step as usize > radius_limit * 2 {
			break;
		}
	}

	offsets
}
