use crate::core::error::GlyphWeaveError;
use crate::layout::common::{
	IncrementalAvailability, available_positions, create_progress_bar, find_fit_at_position,
	finish_progress, occupy_area, pick_color, pick_weighted_word, placement, random_index,
	total_area, update_progress,
};
use crate::layout::{LayoutRequest, LayoutResult, LayoutStrategy};
use rand::RngCore;

const POOL_REFILL_THRESHOLD: usize = 256;

pub struct RandomBaselineStrategy;

impl LayoutStrategy for RandomBaselineStrategy {
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

		let mut availability = IncrementalAvailability::new(&mask);
		// Issue-3: build the position pool once and prune via swap_remove
		// instead of rebuilding the entire pool every iteration (O(W*H)/iter).
		let mut positions = available_positions(&mask);
		let mut placements = Vec::new();
		let mut attempts = 0usize;
		let mut internal_evaluations = 0usize;
		let mut used_area = 0usize;
		let progress = create_progress_bar(request.show_progress);

		while attempts < request.max_try_count {
			let fill_ratio = used_area as f32 / total_usable_area as f32;
			if fill_ratio >= request.ratio_threshold {
				break;
			}

			attempts += 1;

			if positions.is_empty() {
				break;
			}

			let mut placed = false;
			let idx = random_index(rng, positions.len());
			let (y, x) = positions[idx];
			if !mask.get(y, x) {
				positions.swap_remove(idx);
			} else if let Some(word_entry) = pick_weighted_word(request.words, rng) {
				if let Some((font_size, rotation, rect)) = find_fit_at_position(
					&availability,
					&mask,
					x,
					y,
					&word_entry.text,
					request.style,
					request.font,
					&request.text_size_cache,
					&mut internal_evaluations,
				) {
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
					placed = true;
				}
			} else {
				break;
			}

			if !placed && positions.len() < POOL_REFILL_THRESHOLD {
				positions = available_positions(&mask);
			}

			let ratio_progress = (used_area * 100) / total_usable_area;
			let try_progress = (attempts * 100) / request.max_try_count;
			update_progress(&progress, ratio_progress.max(try_progress));
		}

		finish_progress(&progress);

		Ok(LayoutResult {
			placements,
			attempts,
			internal_evaluations,
			used_area,
		})
	}
}
