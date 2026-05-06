use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GlyphWeaveError {
	#[error("invalid configuration: {0}")]
	InvalidConfig(String),

	#[error("font loading failed: {0}")]
	FontLoad(String),

	#[error("I/O error at {path}: {source}", path = path.display())]
	Io {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("image processing error at {path}: {source}", path = path.display())]
	Image {
		path: PathBuf,
		#[source]
		source: image::ImageError,
	},

	#[error("generation failed: {0}")]
	Generation(String),
}
