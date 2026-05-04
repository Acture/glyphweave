use crate::core::error::GlyphWeaveError;
use crate::layout::common::{
	IncrementalAvailability, apply_candidate, available_positions, create_progress_bar,
	finish_progress, pick_color, random_unit_f32, sample_candidate, total_area, update_progress,
};
use crate::layout::{LayoutRequest, LayoutResult, LayoutStrategy};
use rand::RngCore;

const CANDIDATE_TRIALS: usize = 48;
const INITIAL_TEMPERATURE: f32 = 1.0;
const MIN_TEMPERATURE: f32 = 0.02;
const COOLING_RATE: f32 = 0.996;
const POOL_REFILL_THRESHOLD: usize = 256;

pub struct SimulatedAnnealingStrategy;

impl LayoutStrategy for SimulatedAnnealingStrategy {
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
		let mut positions = available_positions(&mask);
		let mut placements = Vec::new();
		let mut attempts = 0usize;
		let mut used_area = 0usize;
		let mut temperature = INITIAL_TEMPERATURE;
		let progress = create_progress_bar(request.show_progress);

		while attempts < request.max_try_count {
			let fill_ratio = used_area as f32 / total_usable_area as f32;
			if fill_ratio >= request.ratio_threshold {
				break;
			}

			attempts += 1;

			if positions.len() < POOL_REFILL_THRESHOLD {
				positions = available_positions(&mask);
			}
			if positions.is_empty() {
				break;
			}

			let Some(candidate) = sample_candidate(
				&mask,
				&availability,
				&mut positions,
				request,
				rng,
				CANDIDATE_TRIALS,
			) else {
				temperature = (temperature * COOLING_RATE).max(MIN_TEMPERATURE);
				continue;
			};

			let candidate_area = candidate.rect.area();
			let normalized_quality = if total_usable_area == 0 {
				0.0
			} else {
				candidate_area as f32 / total_usable_area as f32
			};
			let accepted = accept_candidate(normalized_quality, temperature, rng);

			if accepted {
				let color = pick_color(&request.style.colors, rng);
				let (placed, consumed) = apply_candidate(&mut mask, &candidate, color);
				availability.commit_rect(&mask, candidate.rect);
				used_area += consumed;
				placements.push(placed);
			}

			temperature = (temperature * COOLING_RATE).max(MIN_TEMPERATURE);

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

/// Decide whether to accept a candidate placement under the SA schedule.
///
/// Plan A semantics: energy is the global solution's fill ratio, so accepting
/// a candidate produces `ΔE = candidate_area / total_area ≥ 0`. Because every
/// candidate is a non-negative improvement, plain Metropolis is degenerate
/// (it would always accept). Instead we gate on candidate quality vs. the
/// current temperature:
///
/// * auto-accept when `normalized_quality >= temperature` (the candidate is
///   "big enough" relative to the current annealing stage);
/// * otherwise probabilistic accept with probability `quality / T`,
///   clamped to `[0, 1]`.
///
/// At high T (early) the bar is high → only large words slip through, keeping
/// exploration coarse. As T cools the bar drops → smaller filler words start
/// to be accepted, refining the layout. This realises the classic
/// "coarse-first, fine-later" annealing pattern for area maximisation.
fn accept_candidate(normalized_quality: f32, temperature: f32, rng: &mut dyn RngCore) -> bool {
	if normalized_quality >= temperature {
		return true;
	}
	let acceptance = (normalized_quality / temperature.max(1e-6)).clamp(0.0, 1.0);
	random_unit_f32(rng) < acceptance
}

#[cfg(test)]
mod tests {
	use super::*;
	use rand::SeedableRng;
	use rand::rngs::StdRng;

	#[test]
	fn high_temperature_rejects_small_candidates_more_often_than_low_temperature() {
		// At high T the gate is strict; at low T it relaxes. A small candidate
		// (quality far below 1.0) should therefore see a higher acceptance rate
		// at low T than at high T.
		let small_quality = 0.01_f32;
		let trials = 5_000;
		let mut hot_rng = StdRng::seed_from_u64(0xC0FFEE);
		let mut cold_rng = StdRng::seed_from_u64(0xC0FFEE);

		let mut hot_accepts = 0usize;
		let mut cold_accepts = 0usize;
		for _ in 0..trials {
			if accept_candidate(small_quality, 1.0, &mut hot_rng) {
				hot_accepts += 1;
			}
			if accept_candidate(small_quality, 0.02, &mut cold_rng) {
				cold_accepts += 1;
			}
		}
		assert!(
			cold_accepts > hot_accepts,
			"expected cold acceptance ({cold_accepts}) to exceed hot acceptance ({hot_accepts}) for small quality"
		);
	}

	#[test]
	fn large_candidates_are_always_accepted() {
		let mut rng = StdRng::seed_from_u64(7);
		// quality strictly greater than temperature must auto-accept regardless of RNG.
		for _ in 0..256 {
			assert!(accept_candidate(0.5, 0.4, &mut rng));
			assert!(accept_candidate(1.0, 1.0, &mut rng));
		}
	}

	#[test]
	fn zero_quality_at_high_temperature_never_accepts() {
		// quality = 0 with T > 0 → acceptance probability = 0.
		let mut rng = StdRng::seed_from_u64(42);
		for _ in 0..256 {
			assert!(!accept_candidate(0.0, 0.5, &mut rng));
		}
	}

	#[test]
	fn degenerate_temperature_does_not_panic() {
		// Defensive: a near-zero temperature must not panic and must auto-accept
		// any non-negative quality (since quality >= ~0 trivially).
		let mut rng = StdRng::seed_from_u64(1);
		assert!(accept_candidate(0.001, 0.0, &mut rng));
		assert!(accept_candidate(0.0, 0.0, &mut rng));
	}
}
