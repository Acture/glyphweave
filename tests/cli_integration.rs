use std::process::Command;
use tempfile::tempdir;

fn test_font_path() -> std::path::PathBuf {
	std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/NotoSansSC-Regular.ttf")
}

#[test]
fn cli_generates_svg_file() {
	let dir = tempdir().expect("tempdir should be created");
	let output = dir.path().join("cloud.svg");
	let font = test_font_path();

	let status = Command::new(env!("CARGO_BIN_EXE_glyphweave"))
		.args([
			"--text",
			"RUST",
			"--words",
			"rust,cloud,layout,svg,mask",
			"--seed",
			"42",
			"--algorithm",
			"fast-grid",
			"--font",
		])
		.arg(&font)
		.args(["--no-progress", "--output"])
		.arg(&output)
		.status()
		.expect("process should run");

	assert!(status.success());

	let content = std::fs::read_to_string(&output).expect("svg should be written");
	assert!(content.contains("<svg"));
}

#[test]
fn cli_returns_invalid_config_exit_code_when_words_missing() {
	let dir = tempdir().expect("tempdir should be created");
	let output = dir.path().join("missing.svg");
	let font = test_font_path();

	let result = Command::new(env!("CARGO_BIN_EXE_glyphweave"))
		.args(["--text", "RUST", "--font"])
		.arg(&font)
		.args(["--no-progress", "--output"])
		.arg(&output)
		.output()
		.expect("process should run");

	assert_eq!(result.status.code(), Some(2));
	let stderr = String::from_utf8_lossy(&result.stderr);
	assert!(stderr.contains("no words provided"));
}

#[test]
fn cli_can_write_debug_mask() {
	let dir = tempdir().expect("tempdir should be created");
	let output = dir.path().join("cloud.svg");
	let mask = dir.path().join("mask.png");
	let font = test_font_path();

	let status = Command::new(env!("CARGO_BIN_EXE_glyphweave"))
		.args([
			"--text",
			"RUST",
			"--words",
			"rust,cloud,layout",
			"--canvas-size",
			"420,240",
			"--algorithm",
			"random-baseline",
			"--seed",
			"123",
			"--ratio",
			"0.2",
			"--max-tries",
			"200",
			"--font",
		])
		.arg(&font)
		.args(["--debug-mask-out"])
		.arg(&mask)
		.args(["--no-progress", "--output"])
		.arg(&output)
		.status()
		.expect("process should run");

	assert!(status.success());
	assert!(mask.exists());
}

#[test]
fn cli_rejects_text_and_text_lines_together() {
	let dir = tempdir().expect("tempdir");
	let output = dir.path().join("out.svg");
	let font = test_font_path();

	let result = Command::new(env!("CARGO_BIN_EXE_glyphweave"))
		.args([
			"--text",
			"AI",
			"--text-lines",
			"DATA,SCIENCE",
			"--words",
			"a,b",
			"--font",
		])
		.arg(&font)
		.args(["--no-progress", "--output"])
		.arg(&output)
		.output()
		.expect("process should run");

	assert!(
		!result.status.success(),
		"CLI should reject --text + --text-lines, but exited 0"
	);
	let stderr = String::from_utf8_lossy(&result.stderr);
	assert!(
		stderr.contains("conflict")
			|| stderr.contains("cannot be used")
			|| stderr.contains("text-lines"),
		"stderr should explain mutual exclusion, got: {stderr}"
	);
}

#[test]
fn cli_rejects_shape_image_and_text_lines_together() {
	let dir = tempdir().expect("tempdir");
	let output = dir.path().join("out.svg");
	let font = test_font_path();
	let dummy_image = dir.path().join("dummy.png");
	std::fs::write(
		&dummy_image,
		[
			0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
			0x44, 0x52, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1f, 0x15, 0xc4, 0x89, 0, 0, 0,
			0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0, 0, 0, 0, 2, 0, 1, 0xe5, 0x27, 0xde,
			0xfc, 0, 0, 0, 0, 0x49, 0x45, 0x4E, 0x44, 0xae, 0x42, 0x60, 0x82,
		],
	)
	.ok();

	let result = Command::new(env!("CARGO_BIN_EXE_glyphweave"))
		.args(["--shape-image"])
		.arg(&dummy_image)
		.args(["--text-lines", "DATA,SCIENCE", "--words", "a,b", "--font"])
		.arg(&font)
		.args(["--no-progress", "--output"])
		.arg(&output)
		.output()
		.expect("process should run");

	assert!(
		!result.status.success(),
		"CLI should reject --shape-image + --text-lines"
	);
	let stderr = String::from_utf8_lossy(&result.stderr);
	assert!(
		stderr.contains("conflict") || stderr.contains("cannot be used"),
		"stderr should explain exclusion, got: {stderr}"
	);
}
