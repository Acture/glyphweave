use crate::core::error::GlyphWeaveError;
use fontdue::Font;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CanvasConfig {
	pub width: usize,
	pub height: usize,
	pub margin: usize,
}

impl Default for CanvasConfig {
	fn default() -> Self {
		Self {
			width: 1920,
			height: 1080,
			margin: 10,
		}
	}
}

#[derive(Debug, Clone)]
pub enum FontSizeSpec {
	Fixed(usize),
	AutoFit,
}

#[derive(Debug, Clone)]
pub enum ShapeSource {
	Text {
		text: String,
		font_size: FontSizeSpec,
	},
	Image {
		path: PathBuf,
		threshold: u8,
	},
}

#[derive(Debug, Clone)]
pub struct ShapeConfig {
	pub source: ShapeSource,
}

impl ShapeConfig {
	pub fn text(text: impl Into<String>, font_size: FontSizeSpec) -> Self {
		Self {
			source: ShapeSource::Text {
				text: text.into(),
				font_size,
			},
		}
	}

	pub fn image(path: impl Into<PathBuf>, threshold: u8) -> Self {
		Self {
			source: ShapeSource::Image {
				path: path.into(),
				threshold,
			},
		}
	}
}

#[derive(Debug, Clone)]
pub struct WordEntry {
	pub text: String,
	pub weight: f32,
}

impl WordEntry {
	pub fn new(text: impl Into<String>, weight: f32) -> Self {
		Self {
			text: text.into(),
			weight,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Rotation(pub u16);

#[allow(non_upper_case_globals)]
impl Rotation {
	pub const Deg0: Self = Self(0);
	pub const Deg90: Self = Self(90);
	pub const ZERO: Self = Self(0);
	pub const NINETY: Self = Self(90);

	pub fn degrees(self) -> u16 {
		self.0 % 360
	}

	pub fn radians(self) -> f32 {
		(self.degrees() as f32).to_radians()
	}
}

impl PartialEq for Rotation {
	fn eq(&self, other: &Self) -> bool {
		self.degrees() == other.degrees()
	}
}

impl Eq for Rotation {}

impl std::hash::Hash for Rotation {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.degrees().hash(state);
	}
}

impl serde::Serialize for Rotation {
	fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
		s.serialize_u16(self.degrees())
	}
}

#[derive(Debug, Clone)]
pub struct StyleConfig {
	pub font_size_range: RangeInclusive<usize>,
	pub padding: usize,
	pub colors: Vec<String>,
	pub rotations: Vec<Rotation>,
}

impl Default for StyleConfig {
	fn default() -> Self {
		Self {
			font_size_range: 10..=30,
			padding: 0,
			colors: vec!["#000000".to_string()],
			rotations: vec![Rotation::Deg0],
		}
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum AlgorithmKind {
	RandomBaseline,
	#[default]
	FastGrid,
	SpiralGreedy,
	Mcts,
	SimulatedAnnealing,
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
	pub show_progress: bool,
	pub debug_mask_out: Option<PathBuf>,
}

impl Default for RenderOptions {
	fn default() -> Self {
		Self {
			show_progress: true,
			debug_mask_out: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct CloudRequest {
	pub canvas: CanvasConfig,
	pub shape: ShapeConfig,
	pub words: Vec<WordEntry>,
	pub style: StyleConfig,
	pub algorithm: AlgorithmKind,
	pub ratio_threshold: f32,
	pub max_try_count: usize,
	pub seed: Option<u64>,
	pub font: Arc<Font>,
	pub render: RenderOptions,
}

impl CloudRequest {
	pub fn validate(&self) -> Result<(), GlyphWeaveError> {
		if self.canvas.width == 0 || self.canvas.height == 0 {
			return Err(GlyphWeaveError::InvalidConfig(
				"canvas width/height must be greater than 0".to_string(),
			));
		}

		if self.canvas.margin * 2 >= self.canvas.width
			|| self.canvas.margin * 2 >= self.canvas.height
		{
			return Err(GlyphWeaveError::InvalidConfig(
				"canvas margin is too large for the configured canvas size".to_string(),
			));
		}

		if self.words.is_empty() {
			return Err(GlyphWeaveError::InvalidConfig(
				"at least one word is required".to_string(),
			));
		}

		match &self.shape.source {
			ShapeSource::Text { text, .. } => {
				if text.trim().is_empty() {
					return Err(GlyphWeaveError::InvalidConfig(
						"shape text must not be empty".to_string(),
					));
				}
			}
			ShapeSource::Image { path, .. } => {
				if !path.exists() {
					return Err(GlyphWeaveError::InvalidConfig(format!(
						"shape image path does not exist: {}",
						path.display()
					)));
				}
			}
		}

		if !(0.0..=1.0).contains(&self.ratio_threshold) {
			return Err(GlyphWeaveError::InvalidConfig(
				"ratio threshold must be between 0.0 and 1.0".to_string(),
			));
		}

		if self.max_try_count == 0 {
			return Err(GlyphWeaveError::InvalidConfig(
				"max_try_count must be greater than 0".to_string(),
			));
		}

		let min_size = *self.style.font_size_range.start();
		let max_size = *self.style.font_size_range.end();
		if min_size == 0 || min_size > max_size {
			return Err(GlyphWeaveError::InvalidConfig(
				"font_size_range must be valid and greater than 0".to_string(),
			));
		}

		if self.style.colors.is_empty() {
			return Err(GlyphWeaveError::InvalidConfig(
				"at least one color is required".to_string(),
			));
		}

		if self.style.rotations.is_empty() {
			return Err(GlyphWeaveError::InvalidConfig(
				"at least one rotation is required".to_string(),
			));
		}

		if self.words.iter().any(|w| w.text.trim().is_empty()) {
			return Err(GlyphWeaveError::InvalidConfig(
				"word list contains empty entries".to_string(),
			));
		}

		Ok(())
	}
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CloudPlacement {
	pub word: String,
	pub x: usize,
	pub y: usize,
	pub font_size: usize,
	pub color: String,
	pub rotation: Rotation,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CloudStats {
	pub seed: u64,
	pub shape_font_size: usize,
	pub total_usable_area: usize,
	pub used_area: usize,
	pub fill_ratio: f32,
	pub attempts: usize,
	pub internal_evaluations: usize,
	pub placed_words: usize,
	pub elapsed_ms: u128,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CloudResult {
	pub svg: String,
	pub placements: Vec<CloudPlacement>,
	pub stats: CloudStats,
}
