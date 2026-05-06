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

#[cfg(test)]
mod tests {
	use super::*;
	use std::io;

	#[test]
	fn io_error_display_includes_path() {
		let err = GlyphWeaveError::Io {
			path: PathBuf::from("/tmp/missing.txt"),
			source: io::Error::from(io::ErrorKind::NotFound),
		};
		let msg = format!("{err}");
		assert!(msg.contains("/tmp/missing.txt"), "got: {msg}");
		assert!(
			msg.contains("at "),
			"wording should be 'at {{path}}', got: {msg}"
		);
	}

	#[test]
	fn invalid_config_display_format() {
		let err = GlyphWeaveError::InvalidConfig("bad value".into());
		assert_eq!(format!("{err}"), "invalid configuration: bad value");
	}
}
