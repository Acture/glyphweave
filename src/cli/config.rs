use crate::cli::args::{CliAlgorithm, PaletteKind};
use glyphweave::core::error::GlyphWeaveError;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
	pub canvas_size: Option<[usize; 2]>,
	pub canvas_margin: Option<usize>,
	pub word_size_range: Option<[usize; 2]>,
	pub word_padding: Option<usize>,
	pub colors: Option<Vec<String>>,
	pub rotations: Option<Vec<u16>>,
	pub text_size: Option<String>,
	pub algorithm: Option<String>,
	pub font: Option<PathBuf>,
	pub seed: Option<u64>,
	pub ratio: Option<f32>,
	pub max_tries: Option<usize>,
	pub no_progress: Option<bool>,
	pub palette: Option<String>,
	pub palette_base: Option<String>,
	pub palette_size: Option<usize>,
	pub shape_image_threshold: Option<u8>,
}

impl FileConfig {
	fn merge_from(&mut self, other: FileConfig) {
		if other.canvas_size.is_some() {
			self.canvas_size = other.canvas_size;
		}
		if other.canvas_margin.is_some() {
			self.canvas_margin = other.canvas_margin;
		}
		if other.word_size_range.is_some() {
			self.word_size_range = other.word_size_range;
		}
		if other.word_padding.is_some() {
			self.word_padding = other.word_padding;
		}
		if other.colors.is_some() {
			self.colors = other.colors;
		}
		if other.rotations.is_some() {
			self.rotations = other.rotations;
		}
		if other.text_size.is_some() {
			self.text_size = other.text_size;
		}
		if other.algorithm.is_some() {
			self.algorithm = other.algorithm;
		}
		if other.font.is_some() {
			self.font = other.font;
		}
		if other.seed.is_some() {
			self.seed = other.seed;
		}
		if other.ratio.is_some() {
			self.ratio = other.ratio;
		}
		if other.max_tries.is_some() {
			self.max_tries = other.max_tries;
		}
		if other.no_progress.is_some() {
			self.no_progress = other.no_progress;
		}
		if other.palette.is_some() {
			self.palette = other.palette;
		}
		if other.palette_base.is_some() {
			self.palette_base = other.palette_base;
		}
		if other.palette_size.is_some() {
			self.palette_size = other.palette_size;
		}
		if other.shape_image_threshold.is_some() {
			self.shape_image_threshold = other.shape_image_threshold;
		}
	}

	pub fn canvas_size_tuple(&self) -> Option<(usize, usize)> {
		self.canvas_size.map(|size| (size[0], size[1]))
	}

	pub fn word_size_tuple(&self) -> Option<(usize, usize)> {
		self.word_size_range.map(|size| (size[0], size[1]))
	}

	pub fn algorithm_enum(&self) -> Result<Option<CliAlgorithm>, GlyphWeaveError> {
		let Some(text) = self.algorithm.as_deref() else {
			return Ok(None);
		};

		CliAlgorithm::parse_text(text).map(Some).ok_or_else(|| {
			GlyphWeaveError::InvalidConfig(format!("invalid algorithm '{text}' in config"))
		})
	}

	pub fn palette_enum(&self) -> Result<Option<PaletteKind>, GlyphWeaveError> {
		let Some(text) = self.palette.as_deref() else {
			return Ok(None);
		};

		PaletteKind::parse_text(text).map(Some).ok_or_else(|| {
			GlyphWeaveError::InvalidConfig(format!("invalid palette '{text}' in config"))
		})
	}
}

pub fn load_merged_config(explicit_path: Option<&Path>) -> Result<FileConfig, GlyphWeaveError> {
	let mut merged = FileConfig::default();

	if let Some(path) = user_config_path()
		&& path.exists()
	{
		let file = load_config_file(&path)?;
		merged.merge_from(file);
	}

	let project = PathBuf::from(".glyphweave.toml");
	if project.exists() {
		let file = load_config_file(&project)?;
		merged.merge_from(file);
	}

	if let Some(path) = explicit_path {
		let file = load_config_file(path)?;
		merged.merge_from(file);
	}

	Ok(merged)
}

fn load_config_file(path: &Path) -> Result<FileConfig, GlyphWeaveError> {
	let content = std::fs::read_to_string(path).map_err(|source| GlyphWeaveError::Io {
		path: path.to_path_buf(),
		source,
	})?;

	toml::from_str::<FileConfig>(&content).map_err(|err| {
		GlyphWeaveError::InvalidConfig(format!(
			"failed to parse config '{}': {err}",
			path.display()
		))
	})
}

fn user_config_path() -> Option<PathBuf> {
	if let Ok(xdg_home) = std::env::var("XDG_CONFIG_HOME") {
		let mut path = PathBuf::from(xdg_home);
		path.push("glyphweave");
		path.push("config.toml");
		return Some(path);
	}

	let Ok(home) = std::env::var("HOME") else {
		return None;
	};

	let mut path = PathBuf::from(home);
	path.push(".config");
	path.push("glyphweave");
	path.push("config.toml");
	Some(path)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unknown_field_in_config_is_rejected() {
		let toml_text = r#"
canvas_size = [800, 600]
canva_size = [800, 600]
"#;
		let result: Result<FileConfig, _> = toml::from_str(toml_text);
		assert!(result.is_err(), "expected unknown_field rejection");
		let msg = format!("{}", result.unwrap_err());
		assert!(
			msg.contains("canva_size") || msg.contains("unknown"),
			"error should mention the unknown field, got: {msg}"
		);
	}

	#[test]
	fn known_fields_still_parse() {
		let toml_text = r#"
canvas_size = [1600, 900]
algorithm = "fast-grid"
ratio = 0.85
shape_image_threshold = 200
"#;
		let cfg: FileConfig = toml::from_str(toml_text).expect("known fields should parse");
		assert_eq!(cfg.canvas_size, Some([1600, 900]));
		assert_eq!(cfg.algorithm.as_deref(), Some("fast-grid"));
		assert_eq!(cfg.shape_image_threshold, Some(200));
	}

	#[test]
	fn merge_overrides_shape_image_threshold() {
		let mut base = FileConfig {
			shape_image_threshold: Some(100),
			..FileConfig::default()
		};
		let other = FileConfig {
			shape_image_threshold: Some(200),
			..FileConfig::default()
		};
		base.merge_from(other);
		assert_eq!(base.shape_image_threshold, Some(200));
	}

	#[test]
	fn io_error_includes_path_in_message() {
		let nonexistent = PathBuf::from("/tmp/glyphweave-f8-does-not-exist-1234567890.toml");
		let err = load_merged_config(Some(&nonexistent)).unwrap_err();
		let msg = format!("{err}");
		assert!(
			msg.contains("does-not-exist"),
			"message should include path, got: {msg}"
		);
	}
}
