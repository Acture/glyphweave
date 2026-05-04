mod common;
mod fast_grid;
mod mcts;
mod random_baseline;
mod simulated_annealing;
mod spiral_greedy;
mod text_cache;

use crate::core::error::GlyphWeaveError;
use crate::core::model::{AlgorithmKind, CloudPlacement, StyleConfig, WordEntry};
use fontdue::Font;
use ndarray::Array2;
use rand::RngCore;

pub use fast_grid::FastGridStrategy;
pub use mcts::MctsStrategy;
pub use random_baseline::RandomBaselineStrategy;
pub use simulated_annealing::SimulatedAnnealingStrategy;
pub use spiral_greedy::SpiralGreedyStrategy;
pub use text_cache::TextSizeCache;

#[derive(Debug)]
pub struct LayoutRequest<'a> {
	pub mask: &'a Array2<bool>,
	pub words: &'a [WordEntry],
	pub style: &'a StyleConfig,
	pub font: &'a Font,
	pub ratio_threshold: f32,
	pub max_try_count: usize,
	pub show_progress: bool,
	pub text_size_cache: TextSizeCache,
}

#[derive(Debug)]
pub struct LayoutResult {
	pub placements: Vec<CloudPlacement>,
	pub attempts: usize,
	pub used_area: usize,
}

pub trait LayoutStrategy {
	fn place(
		&self,
		request: &LayoutRequest<'_>,
		rng: &mut dyn RngCore,
	) -> Result<LayoutResult, GlyphWeaveError>;
}

pub fn strategy_for(kind: AlgorithmKind) -> Box<dyn LayoutStrategy> {
	match kind {
		AlgorithmKind::RandomBaseline => Box::new(RandomBaselineStrategy),
		AlgorithmKind::FastGrid => Box::new(FastGridStrategy),
		AlgorithmKind::SpiralGreedy => Box::new(SpiralGreedyStrategy),
		AlgorithmKind::Mcts => Box::new(MctsStrategy),
		AlgorithmKind::SimulatedAnnealing => Box::new(SimulatedAnnealingStrategy),
	}
}
