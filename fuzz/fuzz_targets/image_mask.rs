#![no_main]
use glyphweave::*;
use libfuzzer_sys::fuzz_target;
use std::sync::Arc;

fn font() -> Arc<fontdue::Font> {
	static FONT: std::sync::OnceLock<Arc<fontdue::Font>> = std::sync::OnceLock::new();
	FONT.get_or_init(|| {
		let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.unwrap()
			.join("fonts/NotoSansSC-Regular.ttf");
		Arc::new(load_font_from_file(&path).expect("font"))
	})
	.clone()
}

fuzz_target!(|data: &[u8]| {
	if data.len() < 64 {
		return;
	}
	let tmp = std::env::temp_dir().join(format!(
		"gw_fuzz_{}_{}.png",
		std::process::id(),
		data.len()
	));
	if std::fs::write(&tmp, data).is_err() {
		return;
	}
	let req = CloudRequest {
		canvas: CanvasConfig {
			width: 100,
			height: 100,
			margin: 0,
		},
		shape: ShapeConfig::image(tmp.clone(), 127),
		words: vec![WordEntry::new("ab", 1.0)],
		style: StyleConfig::default(),
		algorithm: AlgorithmKind::FastGrid,
		ratio_threshold: 0.1,
		max_try_count: 50,
		seed: Some(0),
		font: font(),
		render: RenderOptions {
			show_progress: false,
			debug_mask_out: None,
		},
	};
	let _ = generate(req);
	let _ = std::fs::remove_file(&tmp);
});
