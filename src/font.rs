use crate::core::error::GlyphWeaveError;
use fontdue::{Font, FontSettings};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn font_family_name(font: &Font) -> String {
	font.name().unwrap_or("Unknown").to_string()
}

pub fn load_font_from_file<P: AsRef<Path>>(path: P) -> Result<Font, GlyphWeaveError> {
	let path_ref = path.as_ref();
	let font_data = std::fs::read(path_ref).map_err(|err| {
		GlyphWeaveError::FontLoad(format!(
			"failed to read font '{}': {err}",
			path_ref.display()
		))
	})?;

	Font::from_bytes(font_data, FontSettings::default()).map_err(|err| {
		GlyphWeaveError::FontLoad(format!(
			"failed to parse font '{}': {err}",
			path_ref.display()
		))
	})
}

pub fn discover_system_font_candidates() -> Vec<PathBuf> {
	let mut candidates = Vec::new();
	let mut seen = HashSet::new();

	for path in preferred_system_font_paths() {
		push_font_candidate(path, &mut candidates, &mut seen);
	}

	for root in system_font_roots() {
		collect_font_files(&root, 0, 4, &mut candidates, &mut seen);
	}

	candidates.sort_by(|a, b| {
		score_font_path(a)
			.cmp(&score_font_path(b))
			.then_with(|| a.as_os_str().cmp(b.as_os_str()))
	});
	candidates
}

pub fn load_system_font() -> Result<(Font, PathBuf), GlyphWeaveError> {
	let candidates = discover_system_font_candidates();
	load_system_font_from_candidates(&candidates)
}

pub fn load_system_font_from_candidates(
	candidates: &[PathBuf],
) -> Result<(Font, PathBuf), GlyphWeaveError> {
	let mut parse_failures = 0usize;

	for path in candidates {
		match load_font_from_file(path) {
			Ok(font) => return Ok((font, path.clone())),
			Err(_) => parse_failures += 1,
		}
	}

	if candidates.is_empty() {
		return Err(GlyphWeaveError::FontLoad(
			"no system font candidates found; provide --font <path> or enable embedded_fonts feature"
				.to_string(),
		));
	}

	Err(GlyphWeaveError::FontLoad(format!(
		"found {} system font candidates, but none could be parsed ({parse_failures} failures); provide --font <path>",
		candidates.len()
	)))
}

#[cfg(feature = "embedded_fonts")]
pub fn load_default_embedded_font() -> Result<Font, GlyphWeaveError> {
	Font::from_bytes(
		crate::embedded_fonts::NOTO_SANS_SC_REGULAR,
		FontSettings::default(),
	)
	.map_err(|err| GlyphWeaveError::FontLoad(format!("failed to parse embedded font: {err}")))
}

#[cfg(not(feature = "embedded_fonts"))]
pub fn load_default_embedded_font() -> Result<Font, GlyphWeaveError> {
	Err(GlyphWeaveError::FontLoad(
		"no font provided and embedded_fonts feature is disabled".to_string(),
	))
}

fn push_font_candidate(path: PathBuf, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
	if !is_supported_font_file(&path) {
		return;
	}

	if path.exists() && seen.insert(path.clone()) {
		out.push(path);
	}
}

fn collect_font_files(
	dir: &Path,
	depth: usize,
	max_depth: usize,
	out: &mut Vec<PathBuf>,
	seen: &mut HashSet<PathBuf>,
) {
	if depth > max_depth || !dir.exists() {
		return;
	}

	let Ok(entries) = std::fs::read_dir(dir) else {
		return;
	};

	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_font_files(&path, depth + 1, max_depth, out, seen);
			continue;
		}

		push_font_candidate(path, out, seen);
	}
}

fn is_supported_font_file(path: &Path) -> bool {
	let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
		return false;
	};
	matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf")
}

fn preferred_system_font_paths() -> Vec<PathBuf> {
	let mut paths = Vec::new();

	if cfg!(target_os = "macos") {
		paths.push(PathBuf::from("/Library/Fonts/NotoSansSC-Regular.ttf"));
		paths.push(PathBuf::from("/Library/Fonts/SourceHanSansSC-Regular.otf"));
		paths.push(PathBuf::from(
			"/System/Library/Fonts/Supplemental/Arial.ttf",
		));
	}

	if cfg!(target_os = "linux") {
		paths.push(PathBuf::from(
			"/usr/share/fonts/truetype/noto/NotoSansSC-Regular.ttf",
		));
		paths.push(PathBuf::from(
			"/usr/share/fonts/opentype/noto/NotoSansSC-Regular.otf",
		));
		paths.push(PathBuf::from(
			"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
		));
	}

	if cfg!(target_os = "windows")
		&& let Ok(windir) = std::env::var("WINDIR")
	{
		paths.push(PathBuf::from(&windir).join("Fonts").join("arial.ttf"));
		paths.push(PathBuf::from(&windir).join("Fonts").join("segoeui.ttf"));
	}

	if let Ok(custom_font) = std::env::var("GLYPHWEAVE_FONT") {
		paths.insert(0, PathBuf::from(custom_font));
	}

	if let Ok(custom_font) = std::env::var("SHAPECLOUD_FONT") {
		eprintln!("warning: SHAPECLOUD_FONT is deprecated, use GLYPHWEAVE_FONT instead");
		paths.insert(0, PathBuf::from(custom_font));
	}

	paths
}

fn system_font_roots() -> Vec<PathBuf> {
	let mut roots = Vec::new();

	if cfg!(target_os = "macos") {
		roots.push(PathBuf::from("/System/Library/Fonts"));
		roots.push(PathBuf::from("/Library/Fonts"));
		if let Ok(home) = std::env::var("HOME") {
			roots.push(PathBuf::from(home).join("Library/Fonts"));
		}
	}

	if cfg!(target_os = "linux") {
		roots.push(PathBuf::from("/usr/share/fonts"));
		roots.push(PathBuf::from("/usr/local/share/fonts"));
		if let Ok(home) = std::env::var("HOME") {
			roots.push(PathBuf::from(&home).join(".local/share/fonts"));
			roots.push(PathBuf::from(home).join(".fonts"));
		}
	}

	if cfg!(target_os = "windows")
		&& let Ok(windir) = std::env::var("WINDIR")
	{
		roots.push(PathBuf::from(windir).join("Fonts"));
	}

	roots
}

fn score_font_path(path: &Path) -> usize {
	let path_str = path
		.file_name()
		.and_then(|value| value.to_str())
		.unwrap_or_default()
		.to_ascii_lowercase();

	[
		"notosanssc",
		"sourcehansanssc",
		"notosanscjk",
		"sourcehansans",
		"pingfang",
		"hiraginosansgb",
		"microsoftyahei",
		"simhei",
		"simsun",
		"dejavusans",
		"roboto",
		"segoeui",
		"arial",
	]
	.iter()
	.position(|needle| path_str.contains(needle))
	.unwrap_or(usize::MAX)
}
