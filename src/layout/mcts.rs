use crate::core::error::GlyphWeaveError;
use crate::layout::BitMask;
use crate::layout::common::{
	IncrementalAvailability, PlacementCandidate, Rect, apply_candidate, available_positions,
	candidate_quality, create_progress_bar, finish_progress, occupy_area, pick_color,
	sample_candidate, total_area, update_progress,
};
use crate::layout::{LayoutRequest, LayoutResult, LayoutStrategy};
use rand::RngCore;

const CANDIDATE_TRIALS: usize = 64;
const CHILDREN_PER_STEP: usize = 12;
const MCTS_ITERATIONS: usize = 48;
const ROLLOUT_DEPTH: usize = 6;
const ROLLOUT_CANDIDATE_TRIALS: usize = 24;
const UCB_EXPLORATION: f32 = 1.2;
const POOL_REFILL_THRESHOLD: usize = 256;

pub struct MctsStrategy;

#[derive(Debug, Clone)]
struct ChildNode {
	candidate: PlacementCandidate,
	visits: usize,
	total_reward: f32,
}

impl LayoutStrategy for MctsStrategy {
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
		let mut internal_evaluations = 0usize;
		let mut used_area = 0usize;
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

			let mut children = sample_children(
				&mask,
				&availability,
				&mut positions,
				request,
				rng,
				&mut internal_evaluations,
			);
			if children.is_empty() {
				continue;
			}

			for _ in 0..MCTS_ITERATIONS {
				let selected = select_ucb_child(&children);
				let reward = rollout_reward(
					&mut mask,
					&mut availability,
					&positions,
					&children[selected].candidate,
					request,
					total_usable_area,
					rng,
					&mut internal_evaluations,
				);
				let node = &mut children[selected];
				node.visits += 1;
				node.total_reward += reward;
			}

			let best = best_child_index(&children);
			let best_candidate = children.swap_remove(best).candidate;
			let color = pick_color(&request.style.colors, rng);
			let (placed, consumed) = apply_candidate(&mut mask, &best_candidate, color);
			availability.commit_rect(&mask, best_candidate.rect);
			used_area += consumed;
			placements.push(placed);

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

fn sample_children(
	mask: &BitMask,
	availability: &IncrementalAvailability,
	positions: &mut Vec<(usize, usize)>,
	request: &LayoutRequest<'_>,
	rng: &mut dyn RngCore,
	evaluations: &mut usize,
) -> Vec<ChildNode> {
	let mut children = Vec::new();

	for _ in 0..CHILDREN_PER_STEP {
		if let Some(candidate) = sample_candidate(
			mask,
			availability,
			positions,
			request,
			rng,
			CANDIDATE_TRIALS,
			evaluations,
		) {
			children.push(ChildNode {
				candidate,
				visits: 0,
				total_reward: 0.0,
			});
		}
	}

	children
}

fn select_ucb_child(children: &[ChildNode]) -> usize {
	if let Some((index, _)) = children
		.iter()
		.enumerate()
		.find(|(_, child)| child.visits == 0)
	{
		return index;
	}

	let total_visits = children.iter().map(|child| child.visits).sum::<usize>() as f32;

	let mut best_idx = 0usize;
	let mut best_score = f32::NEG_INFINITY;

	for (idx, child) in children.iter().enumerate() {
		let mean = child.total_reward / child.visits as f32;
		let exploration = UCB_EXPLORATION * ((total_visits.ln()) / child.visits as f32).sqrt();
		let score = mean + exploration;
		if score > best_score {
			best_score = score;
			best_idx = idx;
		}
	}

	best_idx
}

fn best_child_index(children: &[ChildNode]) -> usize {
	let mut best_idx = 0usize;
	let mut best_score = f32::NEG_INFINITY;

	for (idx, child) in children.iter().enumerate() {
		let avg = if child.visits == 0 {
			0.0
		} else {
			child.total_reward / child.visits as f32
		};
		if avg > best_score {
			best_score = avg;
			best_idx = idx;
		}
	}

	best_idx
}

#[allow(clippy::too_many_arguments)]
fn rollout_reward(
	mask: &mut BitMask,
	availability: &mut IncrementalAvailability,
	parent_positions: &[(usize, usize)],
	first: &PlacementCandidate,
	request: &LayoutRequest<'_>,
	total_usable_area: usize,
	rng: &mut dyn RngCore,
	evaluations: &mut usize,
) -> f32 {
	let snapshot = availability.snapshot();
	let mut rollout_rects: Vec<Rect> = Vec::with_capacity(ROLLOUT_DEPTH + 1);

	let first_consumed = occupy_area(mask, first.rect);
	availability.commit_rect(mask, first.rect);
	rollout_rects.push(first.rect);

	let mut reward = 0.0f32;
	reward += first_consumed as f32 / total_usable_area as f32;
	reward += candidate_quality(first, total_usable_area);

	// Borrow the parent's position pool instead of re-scanning the mask each
	// rollout step. We work on a small clone so swap_remove inside
	// sample_candidate doesn't mutate the parent pool.
	let mut local_positions: Vec<(usize, usize)> = parent_positions.to_vec();

	for _ in 0..ROLLOUT_DEPTH {
		if local_positions.is_empty() {
			// Fallback rescan: only happens if the borrowed pool was already
			// empty entering the rollout (rare; matches old semantics).
			local_positions = available_positions(mask);
			if local_positions.is_empty() {
				break;
			}
		}

		let Some(candidate) = sample_candidate(
			mask,
			availability,
			&mut local_positions,
			request,
			rng,
			ROLLOUT_CANDIDATE_TRIALS,
			evaluations,
		) else {
			break;
		};

		let consumed = occupy_area(mask, candidate.rect);
		availability.commit_rect(mask, candidate.rect);
		rollout_rects.push(candidate.rect);
		reward += consumed as f32 / total_usable_area as f32;
	}

	// Roll the mask back: re-set every cell of every placed rect. Order
	// doesn't matter for correctness because rollout rects don't overlap
	// (sample_candidate enforced availability at placement time).
	for rect in rollout_rects.iter().rev() {
		let row_end = (rect.y + rect.h).min(mask.nrows());
		let col_end = (rect.x + rect.w).min(mask.ncols());
		for y in rect.y..row_end {
			for x in rect.x..col_end {
				mask.set(y, x, true);
			}
		}
	}
	availability.restore(mask, snapshot);

	reward
}
