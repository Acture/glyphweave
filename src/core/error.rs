use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GlyphWeaveError {
	#[error("invalid configuration: {0}")]
	InvalidConfig(String),

	#[error("font loading failed: {0}")]
	FontLoad(String),

	#[error("I/O error reading {path}: {source}", path = path.display())]
	Io {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("image error writing {path}: {source}", path = path.display())]
	Image {
		path: PathBuf,
		#[source]
		source: image::ImageError,
	},

	#[error("generation failed: {0}")]
	Generation(String),
}
