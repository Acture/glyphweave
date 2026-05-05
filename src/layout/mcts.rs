use crate::core::error::GlyphWeaveError;
use crate::layout::BitMask;
use crate::layout::common::{
	IncrementalAvailability, PlacementCandidate, apply_candidate, available_positions,
	candidate_quality, create_progress_bar, finish_progress, pick_color, sample_candidate,
	total_area, update_progress,
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
					&mask,
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

fn rollout_reward(
	mask: &BitMask,
	first: &PlacementCandidate,
	request: &LayoutRequest<'_>,
	total_usable_area: usize,
	rng: &mut dyn RngCore,
	evaluations: &mut usize,
) -> f32 {
	let mut local_mask = mask.clone();
	let mut reward = 0.0f32;

	let first_consumed = crate::layout::common::occupy_area(&mut local_mask, first.rect);
	// Rollout uses its own availability; it rebuilds the integral once per
	// rollout (P7 will replace the rollout entirely with a cheaper estimator).
	let mut local_availability = IncrementalAvailability::new(&local_mask);
	reward += first_consumed as f32 / total_usable_area as f32;
	reward += candidate_quality(first, total_usable_area);

	for _ in 0..ROLLOUT_DEPTH {
		let mut positions = available_positions(&local_mask);
		if positions.is_empty() {
			break;
		}

		let Some(candidate) = sample_candidate(
			&local_mask,
			&local_availability,
			&mut positions,
			request,
			rng,
			ROLLOUT_CANDIDATE_TRIALS,
			evaluations,
		) else {
			break;
		};

		let consumed = crate::layout::common::occupy_area(&mut local_mask, candidate.rect);
		local_availability.commit_rect(&local_mask, candidate.rect);
		reward += consumed as f32 / total_usable_area as f32;
	}

	reward
}
