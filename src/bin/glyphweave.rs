#[path = "../cli/mod.rs"]
mod cli;

use clap::Parser;
use cli::args::{CliAlgorithm, CliArgs, PaletteKind, collect_words, parse_shape_size_text};
use cli::config::load_merged_config;
use cli::palette::resolve_colors;
use env_logger::Builder;
use glyphweave::core::error::GlyphWeaveError;
use glyphweave::core::model::{
	CanvasConfig, CloudRequest, FontSizeSpec, RenderOptions, ShapeConfig, StyleConfig,
};
use glyphweave::font::{discover_system_font_candidates, load_system_font_from_candidates};
use glyphweave::{
	generate, load_default_embedded_font, load_font_from_file, rotations_from_degrees,
};
use log::{error, info};
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

const DEFAULT_CANVAS_SIZE: (usize, usize) = (1920, 1080);
const DEFAULT_CANVAS_MARGIN: usize = 10;
const DEFAULT_WORD_SIZE_RANGE: (usize, usize) = (10, 30);
const DEFAULT_WORD_PADDING: usize = 0;
const DEFAULT_RATIO: f32 = 0.9;
const DEFAULT_MAX_TRIES: usize = 10_000;
const DEFAULT_PALETTE_BASE: &str = "#3B82F6";
const DEFAULT_PALETTE_SIZE: usize = 6;

fn main() -> ExitCode {
	let args = CliArgs::parse();

	setup_logging(args.verbose);

	match run(args) {
		Ok(()) => ExitCode::SUCCESS,
		Err(err) => {
			let code = map_error_to_exit_code(&err);
			error!("{err}");
			ExitCode::from(code)
		}
	}
}

fn run(args: CliArgs) -> Result<(), GlyphWeaveError> {
	let config = load_merged_config(args.config.as_deref())?;

	let canvas_size = args
		.canvas_size
		.or_else(|| config.canvas_size_tuple())
		.unwrap_or(DEFAULT_CANVAS_SIZE);
	let canvas_margin = args
		.canvas_margin
		.or(config.canvas_margin)
		.unwrap_or(DEFAULT_CANVAS_MARGIN);
	let word_size_range = args
		.word_size_range
		.or_else(|| config.word_size_tuple())
		.unwrap_or(DEFAULT_WORD_SIZE_RANGE);
	let word_padding = args
		.word_padding
		.or(config.word_padding)
		.unwrap_or(DEFAULT_WORD_PADDING);
	let rotations = args
		.rotations
		.clone()
		.or_else(|| config.rotations.clone())
		.unwrap_or_else(|| vec![0]);

	let shape_size = match (&args.shape_size, &config.text_size) {
		(Some(size), _) => size.clone(),
		(None, Some(size_text)) => parse_shape_size_text(size_text).map_err(|err| {
			GlyphWeaveError::InvalidConfig(format!("invalid text_size in config: {err}"))
		})?,
		(None, None) => FontSizeSpec::AutoFit,
	};

	let algorithm = args
		.algorithm
		.or(config.algorithm_enum()?)
		.unwrap_or(CliAlgorithm::FastGrid);

	let palette = args
		.palette
		.or(config.palette_enum()?)
		.unwrap_or(PaletteKind::Auto);
	let palette_base = args
		.palette_base
		.clone()
		.or(config.palette_base.clone())
		.unwrap_or_else(|| DEFAULT_PALETTE_BASE.to_string());
	let palette_size = args
		.palette_size
		.or(config.palette_size)
		.unwrap_or(DEFAULT_PALETTE_SIZE);

	let colors = resolve_colors(
		args.word_colors.clone().or(config.colors.clone()),
		palette,
		&palette_base,
		palette_size,
	)?;

	let ratio = args.threshold.or(config.ratio).unwrap_or(DEFAULT_RATIO);
	let max_tries = args
		.max_tries
		.or(config.max_tries)
		.unwrap_or(DEFAULT_MAX_TRIES);
	let seed = args.seed.or(config.seed);
	let no_progress = if args.no_progress {
		true
	} else {
		config.no_progress.unwrap_or(false)
	};

	let font_path = args.font_path.as_ref().or(config.font.as_ref());
	let font = if let Some(path) = font_path {
		info!("Using user-provided font: {}", path.display());
		load_font_from_file(path)?
	} else {
		match load_default_embedded_font() {
			Ok(font) => {
				info!("Using embedded Noto Sans SC font");
				font
			}
			Err(embedded_error) => {
				let candidates = discover_system_font_candidates();
				let (font, selected) = if args.choose_system_font {
					let selected =
						select_system_font_candidate(&candidates, args.choose_system_font)?;
					let font = load_font_from_file(&selected)?;
					(font, selected)
				} else {
					load_system_font_from_candidates(&candidates)?
				};
				info!(
					"Embedded font unavailable ({}), using system font: {}",
					embedded_error,
					selected.display()
				);
				font
			}
		}
	};

	let words = collect_words(&args)?;

	let request = CloudRequest {
		canvas: CanvasConfig {
			width: canvas_size.0,
			height: canvas_size.1,
			margin: canvas_margin,
		},
		shape: ShapeConfig {
			text: args.shape_text,
			font_size: shape_size,
		},
		words,
		style: StyleConfig {
			font_size_range: word_size_range.0..=word_size_range.1,
			padding: word_padding,
			colors,
			rotations: rotations_from_degrees(&rotations)?,
		},
		algorithm: algorithm.into(),
		ratio_threshold: ratio,
		max_try_count: max_tries,
		seed,
		font: Arc::new(font),
		render: RenderOptions {
			show_progress: !no_progress,
			debug_mask_out: args.debug_mask_out,
		},
	};

	let output_path = args.output;
	let result = generate(request)?;
	std::fs::write(&output_path, result.svg).map_err(|source| GlyphWeaveError::Io {
		path: output_path.clone(),
		source,
	})?;

	info!(
		"Generated {} words, fill ratio {:.2}% (seed={}) -> {}",
		result.stats.placed_words,
		result.stats.fill_ratio * 100.0,
		result.stats.seed,
		output_path.display()
	);

	eprintln!(
		"glyphweave: placed {} words, fill {:.1}%, seed={}, attempts={}, internal_evals={}, elapsed {}ms -> {}",
		result.stats.placed_words,
		result.stats.fill_ratio * 100.0,
		result.stats.seed,
		result.stats.attempts,
		result.stats.internal_evaluations,
		result.stats.elapsed_ms,
		output_path.display()
	);

	Ok(())
}

fn map_error_to_exit_code(error: &GlyphWeaveError) -> u8 {
	match error {
		GlyphWeaveError::InvalidConfig(_) => 2,
		GlyphWeaveError::FontLoad(_) => 3,
		GlyphWeaveError::Io { .. } | GlyphWeaveError::Image { .. } => 4,
		GlyphWeaveError::Generation(_) => 5,
	}
}

fn setup_logging(verbose: bool) {
	let level = if cfg!(debug_assertions) {
		if verbose {
			log::LevelFilter::Debug
		} else {
			log::LevelFilter::Info
		}
	} else if verbose {
		log::LevelFilter::Debug
	} else {
		log::LevelFilter::Warn
	};

	Builder::from_default_env()
		.filter_level(level)
		.format_level(true)
		.format_timestamp_secs()
		.format_module_path(false)
		.format_target(false)
		.init();
}

fn select_system_font_candidate(
	candidates: &[PathBuf],
	interactive: bool,
) -> Result<PathBuf, GlyphWeaveError> {
	if candidates.is_empty() {
		return Err(GlyphWeaveError::FontLoad(
			"no system fonts discovered; provide --font <path> or build with --features embedded_fonts"
				.to_string(),
		));
	}

	if !interactive
		|| candidates.len() == 1
		|| !std::io::stdin().is_terminal()
		|| !std::io::stderr().is_terminal()
	{
		return Ok(candidates[0].clone());
	}

	let display_count = candidates.len().min(15);
	eprintln!("Embedded font unavailable. Choose a system font:");
	for (index, path) in candidates.iter().take(display_count).enumerate() {
		eprintln!("  [{}] {}", index + 1, path.display());
	}
	if candidates.len() > display_count {
		eprintln!(
			"  ... {} more candidates not shown",
			candidates.len() - display_count
		);
	}

	eprint!("Select [1-{display_count}] (default 1): ");
	let _ = std::io::stderr().flush();

	let mut input = String::new();
	if std::io::stdin().read_line(&mut input).is_err() {
		return Ok(candidates[0].clone());
	}

	let trimmed = input.trim();
	if trimmed.is_empty() {
		return Ok(candidates[0].clone());
	}

	let selected = trimmed.parse::<usize>().map_err(|_| {
		GlyphWeaveError::InvalidConfig(format!("invalid system font selection '{trimmed}'"))
	})?;
	if !(1..=display_count).contains(&selected) {
		return Err(GlyphWeaveError::InvalidConfig(format!(
			"system font selection out of range: {selected}"
		)));
	}

	Ok(candidates[selected - 1].clone())
}
