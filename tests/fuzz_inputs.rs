//! Hand-rolled fuzz harness for input parsers.
//!
//! Feeds deterministic pseudo-random byte sequences through user-input
//! parsers and asserts that none of them panic or terminate by signal.
//! 50 byte sequences are fed to the `--word-file` CLI parser, and 50
//! random PNG buffers to the image-mask loader.
//! This is narrower coverage than `cargo-fuzz`/libFuzzer would give us,
//! but it runs as part of the normal `cargo test` invocation on stable
//! Rust, so regressions are caught in CI without separate infrastructure.

use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use glyphweave::{
	AlgorithmKind, CanvasConfig, CloudRequest, RenderOptions, ShapeConfig, StyleConfig, WordEntry,
	generate, load_font_from_file,
};
use image::ImageBuffer;
use tempfile::{NamedTempFile, tempdir};

/// Xorshift64* — pure-std reproducible PRNG. Avoids pulling rand into
/// the test crate just for fuzz seeding, and keeps each (seed, len)
/// pair reproducible across runs.
fn deterministic_random_bytes(seed: u64, len: usize) -> Vec<u8> {
	let mut state = seed | 1;
	(0..len)
		.map(|_| {
			state ^= state << 13;
			state ^= state >> 7;
			state ^= state << 17;
			(state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 56) as u8
		})
		.collect()
}

fn font_path() -> std::path::PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/NotoSansSC-Regular.ttf")
}

#[test]
fn parse_word_file_does_not_panic_on_random_input() {
	let bin = env!("CARGO_BIN_EXE_glyphweave");
	let font = font_path();
	let out_dir = tempdir().expect("tempdir should be created");
	let out_path = out_dir.path().join("cloud.svg");

	for seed in 0u64..50 {
		let bytes = deterministic_random_bytes(seed, 256);
		let mut tmp = NamedTempFile::new().expect("tempfile should be created");
		tmp.write_all(&bytes).expect("write fuzz input");
		tmp.flush().expect("flush fuzz input");

		let output = Command::new(bin)
			.args(["--text", "AI", "--word-file"])
			.arg(tmp.path())
			.arg("--font")
			.arg(&font)
			.args(["--no-progress", "--seed", "0", "--max-tries", "10"])
			.arg("--output")
			.arg(&out_path)
			.output()
			.expect("CLI should run");

		match output.status.code() {
			Some(code) => assert!(
				matches!(code, 0 | 2 | 3 | 4 | 5),
				"unexpected CLI exit code {code} for seed {seed}; stderr={}",
				String::from_utf8_lossy(&output.stderr)
			),
			None => panic!(
				"CLI killed by signal on seed {seed}: {:?}; stderr={}",
				output.status,
				String::from_utf8_lossy(&output.stderr)
			),
		}
	}
}

#[test]
fn build_image_mask_does_not_panic_on_random_pixels() {
	let font = Arc::new(load_font_from_file(font_path()).expect("test font should load"));
	let dir = tempdir().expect("tempdir should be created");

	const W: u32 = 100;
	const H: u32 = 100;

	for seed in 0u64..50 {
		let bytes = deterministic_random_bytes(seed, (W * H * 4) as usize);
		let img: ImageBuffer<image::Rgba<u8>, _> =
			ImageBuffer::from_raw(W, H, bytes).expect("buffer length matches dimensions");
		let path = dir.path().join(format!("mask_{seed}.png"));
		img.save(&path).expect("png write");

		let request = CloudRequest {
			canvas: CanvasConfig {
				width: W as usize,
				height: H as usize,
				margin: 0,
			},
			shape: ShapeConfig::image(path, 127),
			words: vec![WordEntry::new("ab", 1.0), WordEntry::new("cd", 0.5)],
			style: StyleConfig::default(),
			algorithm: AlgorithmKind::FastGrid,
			ratio_threshold: 0.1,
			max_try_count: 20,
			seed: Some(seed),
			font: Arc::clone(&font),
			render: RenderOptions {
				show_progress: false,
				debug_mask_out: None,
			},
		};
		// Either Ok or graceful Err — never a panic.
		let _ = generate(request);
	}
}
