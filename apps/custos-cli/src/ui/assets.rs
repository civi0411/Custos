//! Asset integration and ultra-sharp high-definition terminal rasterizer for Custos CLI
//!
//! Direct integration of raw image assets from `frame-ui/`:
//! - `owl.png` / `owl.jpg`: Project mascot standing beside Custos banner
//! - `custos-owl-coder.png`: Task Idle and Task Running / Coding
//! - `custos-owl-inspector.png`: Repo analysis, verification, checking evidence, waiting for approval
//! - `custos-owl-steward.png`: Task completed successfully
//!
//! Features:
//! - Lanczos3 high-order resampling
//! - Laplacian unsharp masking (3x3 convolution) for crystal clear lineart & details
//! - Crisp edge alpha thresholding to eliminate fuzzy halos
//! - Vibrance & dynamic range enhancement for luminous TrueColor terminal rendering

use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Rgba};
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::OnceLock;

pub const ASSET_MASCOT_BYTES: &[u8] = include_bytes!("../frame-ui/owl.png");
pub const ASSET_CODER_BYTES: &[u8] = include_bytes!("../frame-ui/custos-owl-coder.png");
pub const ASSET_INSPECTOR_BYTES: &[u8] = include_bytes!("../frame-ui/custos-owl-inspector.png");
pub const ASSET_STEWARD_BYTES: &[u8] = include_bytes!("../frame-ui/custos-owl-steward.png");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    /// Mascot owl standing beside Custos banner (owl.png)
    Mascot,
    /// Coder owl: Task idle or Task running/coding (custos-owl-coder.png)
    Coder,
    /// Inspector owl: Analysis, verification, checking evidence, awaiting approval (custos-owl-inspector.png)
    Inspector,
    /// Steward owl: Task completed successfully (custos-owl-steward.png)
    Steward,
}

impl AssetKind {
    pub fn raw_bytes(&self) -> &'static [u8] {
        match self {
            AssetKind::Mascot => ASSET_MASCOT_BYTES,
            AssetKind::Coder => ASSET_CODER_BYTES,
            AssetKind::Inspector => ASSET_INSPECTOR_BYTES,
            AssetKind::Steward => ASSET_STEWARD_BYTES,
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            AssetKind::Mascot => "owl.png",
            AssetKind::Coder => "custos-owl-coder.png",
            AssetKind::Inspector => "custos-owl-inspector.png",
            AssetKind::Steward => "custos-owl-steward.png",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            AssetKind::Mascot => "Custos Guardian Mascot",
            AssetKind::Coder => "Custos Coder Owl",
            AssetKind::Inspector => "Custos Inspector Owl",
            AssetKind::Steward => "Custos Steward Owl",
        }
    }
}

/// Decode raw image bytes from memory
pub fn load_asset_image(asset: AssetKind) -> Result<DynamicImage, image::ImageError> {
    image::load_from_memory(asset.raw_bytes())
}

/// Applies connected flood-fill on outer white background to isolate foreground owl mascot,
/// then performs multi-pass color bleeding (dilation) onto nearby background pixels.
///
/// Setting nearby transparent pixels to match the owl edge color (with Alpha = 0)
/// guarantees that Lanczos3 resampling averages foreground with foreground,
/// 100% eliminating ugly dark/black fringes and white halo artifacts.
fn make_outer_background_transparent_and_bleed(img: &DynamicImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return rgba;
    }

    let mut visited = vec![false; (w * h) as usize];
    let mut q = VecDeque::new();

    // Seed outer edge pixels
    for x in 0..w {
        for &y in &[0, h - 1] {
            let p = rgba.get_pixel(x, y);
            if p[0] > 235 && p[1] > 235 && p[2] > 235 {
                let idx = (y * w + x) as usize;
                if !visited[idx] {
                    visited[idx] = true;
                    q.push_back((x, y));
                }
            }
        }
    }

    for y in 0..h {
        for &x in &[0, w - 1] {
            let p = rgba.get_pixel(x, y);
            if p[0] > 235 && p[1] > 235 && p[2] > 235 {
                let idx = (y * w + x) as usize;
                if !visited[idx] {
                    visited[idx] = true;
                    q.push_back((x, y));
                }
            }
        }
    }

    while let Some((cx, cy)) = q.pop_front() {
        for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                let ux = nx as u32;
                let uy = ny as u32;
                let idx = (uy * w + ux) as usize;
                if !visited[idx] {
                    let p = rgba.get_pixel(ux, uy);
                    if p[0] > 222 && p[1] > 222 && p[2] > 222 {
                        visited[idx] = true;
                        q.push_back((ux, uy));
                    }
                }
            }
        }
    }

    // Color bleed (dilation): For background pixels immediately next to foreground,
    // copy the edge pixel's RGB while marking alpha = 0.
    // This provides clean interpolation boundaries for Lanczos3.
    let original = rgba.clone();
    for (idx, &is_outer_bg) in visited.iter().enumerate() {
        let ux = (idx as u32) % w;
        let uy = (idx as u32) / w;
        if is_outer_bg {
            // Check 4-connected neighbors for foreground color
            let mut nearest_fg = None;
            for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1), (-2, 0), (2, 0), (0, -2), (0, 2)] {
                let nx = ux as i32 + dx;
                let ny = uy as i32 + dy;
                if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                    let n_idx = (ny as u32 * w + nx as u32) as usize;
                    if !visited[n_idx] {
                        nearest_fg = Some(*original.get_pixel(nx as u32, ny as u32));
                        break;
                    }
                }
            }
            if let Some(mut fg_col) = nearest_fg {
                fg_col[3] = 0; // Transparent alpha, but authentic edge RGB
                rgba.put_pixel(ux, uy, fg_col);
            } else {
                rgba.put_pixel(ux, uy, Rgba([0, 0, 0, 0]));
            }
        }
    }

    rgba
}

/// Automatically crops away empty transparent padding so owl assets occupy 100% of the allocated terminal space.
fn autocrop_content(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return img.clone();
    }

    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            if p[3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }

    if !found {
        return img.clone();
    }

    let pad = 3;
    let crop_x = min_x.saturating_sub(pad);
    let crop_y = min_y.saturating_sub(pad);
    let crop_w = (max_x + 1 + pad).min(w) - crop_x;
    let crop_h = (max_y + 1 + pad).min(h) - crop_y;

    image::imageops::crop_imm(img, crop_x, crop_y, crop_w, crop_h).to_image()
}

/// Pure edge-preserving Laplacian unsharp filter.
/// Sharpens fine lineart, eye contours, and feathers while strictly preserving 100% authentic original artwork colors.
fn sharpen_and_enhance(
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    strength: f32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (w, h) = img.dimensions();
    if w < 3 || h < 3 {
        return img.clone();
    }

    let mut out = img.clone();
    for y in 0..h {
        for x in 0..w {
            let c = img.get_pixel(x, y);
            if c[3] < 30 {
                out.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                continue;
            }

            if x == 0 || x == w - 1 || y == 0 || y == h - 1 {
                continue;
            }

            let top = img.get_pixel(x, y - 1);
            let bot = img.get_pixel(x, y + 1);
            let left = img.get_pixel(x - 1, y);
            let right = img.get_pixel(x + 1, y);

            let mut new_c = *c;
            for i in 0..3 {
                let center_f = c[i] as f32;
                // Transparent-aware neighbor sampling: clamp to center pixel if neighbor is background
                let top_val = if top[3] > 25 { top[i] as f32 } else { center_f };
                let bot_val = if bot[3] > 25 { bot[i] as f32 } else { center_f };
                let left_val = if left[3] > 25 { left[i] as f32 } else { center_f };
                let right_val = if right[3] > 25 { right[i] as f32 } else { center_f };

                let lap = 4.0 * center_f - (top_val + bot_val + left_val + right_val);
                let sharp = (center_f + strength * lap).clamp(0.0, 255.0);
                new_c[i] = sharp as u8;
            }

            // Authentic original colors are strictly preserved (no artificial saturation or tinting)
            out.put_pixel(x, y, new_c);
        }
    }
    out
}

/// Rasterizes an integrated asset image into terminal lines using ANSI 24-bit TrueColor half-blocks (`▀` and `▄`).
/// Preserves authentic original artwork colors while providing sharp pixel boundaries.
pub fn render_asset_to_lines(asset: AssetKind, target_w: u32, target_h: u32) -> Vec<String> {
    let img = match load_asset_image(asset) {
        Ok(i) => i,
        Err(_) => return Vec::new(),
    };

    let trans_img = make_outer_background_transparent_and_bleed(&img);
    let cropped = autocrop_content(&trans_img);
    let resized = image::imageops::resize(&cropped, target_w, target_h, FilterType::Lanczos3);
    let enhanced = sharpen_and_enhance(&resized, 0.40);

    let mut lines = Vec::with_capacity((target_h as usize + 1) / 2);

    for row in (0..target_h).step_by(2) {
        let mut line = String::with_capacity(target_w as usize * 32);

        for col in 0..target_w {
            let p1 = enhanced.get_pixel(col, row);
            let p2 = if row + 1 < target_h {
                enhanced.get_pixel(col, row + 1)
            } else {
                &Rgba([0, 0, 0, 0])
            };

            let top_vis = p1[3] >= 65;
            let bot_vis = p2[3] >= 65;

            match (top_vis, bot_vis) {
                (false, false) => {
                    line.push(' ');
                }
                (false, true) => {
                    let _ = write!(
                        line,
                        "\x1b[38;2;{};{};{}m▄\x1b[0m",
                        p2[0], p2[1], p2[2]
                    );
                }
                (true, false) => {
                    let _ = write!(
                        line,
                        "\x1b[38;2;{};{};{}m▀\x1b[0m",
                        p1[0], p1[1], p1[2]
                    );
                }
                (true, true) => {
                    let _ = write!(
                        line,
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀\x1b[0m",
                        p1[0], p1[1], p1[2], p2[0], p2[1], p2[2]
                    );
                }
            }
        }

        lines.push(line);
    }

    lines
}

// Cached rendered lines for banner mascot (44 width x 52 height = 26 terminal lines)
static MASCOT_BANNER_CACHE: OnceLock<Vec<String>> = OnceLock::new();

// Mini rendered lines for small/narrow showcase (14 cols x 18 height = 9 terminal lines)
static SHOWCASE_CODER_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_INSPECTOR_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_STEWARD_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();

// Medium rendered lines for medium showcase (18 cols x 22 height = 11 terminal lines)
static SHOWCASE_CODER_MED_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_INSPECTOR_MED_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_STEWARD_MED_CACHE: OnceLock<Vec<String>> = OnceLock::new();

// Wide/Compact rendered lines for side-by-side showcase (20 cols x 24 height = 12 terminal lines)
static SHOWCASE_CODER_COMPACT_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_INSPECTOR_COMPACT_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_STEWARD_COMPACT_CACHE: OnceLock<Vec<String>> = OnceLock::new();

// Rendered lines for mode activation card (20 cols x 24 height = 12 terminal lines)
static MODE_CARD_CODER_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_INSPECTOR_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_STEWARD_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();

// Compact rendered lines for task lifecycle events (20 cols x 24 height = 12 terminal lines)
static LIFECYCLE_CODER_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static LIFECYCLE_INSPECTOR_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static LIFECYCLE_STEWARD_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static HD_CODER_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static HD_INSPECTOR_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static HD_STEWARD_CACHE: OnceLock<Vec<String>> = OnceLock::new();

/// Banner Mascot standing beside Custos banner (44 cols x 52 height = 26 terminal lines, 2,288 pixels)
pub fn get_mascot_banner_lines() -> &'static [String] {
    MASCOT_BANNER_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Mascot, 44, 52))
        .as_slice()
}

/// Mini Showcase Coder Owl (14 cols x 18 height = 9 lines)
pub fn get_showcase_coder_mini_lines() -> &'static [String] {
    SHOWCASE_CODER_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 14, 18))
        .as_slice()
}

/// Mini Showcase Inspector Owl (14 cols x 18 height = 9 lines)
pub fn get_showcase_inspector_mini_lines() -> &'static [String] {
    SHOWCASE_INSPECTOR_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 14, 18))
        .as_slice()
}

/// Mini Showcase Steward Owl (14 cols x 18 height = 9 lines)
pub fn get_showcase_steward_mini_lines() -> &'static [String] {
    SHOWCASE_STEWARD_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 14, 18))
        .as_slice()
}

/// Medium Showcase Coder Owl (18 cols x 22 height = 11 lines)
pub fn get_showcase_coder_med_lines() -> &'static [String] {
    SHOWCASE_CODER_MED_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 18, 22))
        .as_slice()
}

/// Medium Showcase Inspector Owl (18 cols x 22 height = 11 lines)
pub fn get_showcase_inspector_med_lines() -> &'static [String] {
    SHOWCASE_INSPECTOR_MED_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 18, 22))
        .as_slice()
}

/// Medium Showcase Steward Owl (18 cols x 22 height = 11 lines)
pub fn get_showcase_steward_med_lines() -> &'static [String] {
    SHOWCASE_STEWARD_MED_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 18, 22))
        .as_slice()
}

/// Compact/Wide Showcase Coder Owl (20 cols x 24 height = 12 lines)
pub fn get_showcase_coder_compact_lines() -> &'static [String] {
    SHOWCASE_CODER_COMPACT_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 20, 24))
        .as_slice()
}

/// Compact/Wide Showcase Inspector Owl (20 cols x 24 height = 12 lines)
pub fn get_showcase_inspector_compact_lines() -> &'static [String] {
    SHOWCASE_INSPECTOR_COMPACT_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 20, 24))
        .as_slice()
}

/// Compact/Wide Showcase Steward Owl (20 cols x 24 height = 12 lines)
pub fn get_showcase_steward_compact_lines() -> &'static [String] {
    SHOWCASE_STEWARD_COMPACT_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 20, 24))
        .as_slice()
}

/// Mode Card Coder Owl (20 cols x 24 height = 12 lines)
pub fn get_mode_card_coder_mini_lines() -> &'static [String] {
    MODE_CARD_CODER_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 20, 24))
        .as_slice()
}

/// Mode Card Inspector Owl (20 cols x 24 height = 12 lines)
pub fn get_mode_card_inspector_mini_lines() -> &'static [String] {
    MODE_CARD_INSPECTOR_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 20, 24))
        .as_slice()
}

/// Mode Card Steward Owl (20 cols x 24 height = 12 lines)
pub fn get_mode_card_steward_mini_lines() -> &'static [String] {
    MODE_CARD_STEWARD_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 20, 24))
        .as_slice()
}

/// Compact Mode Card Coder Owl (20 cols x 24 height = 12 lines)
pub fn get_mode_card_coder_lines() -> &'static [String] {
    get_mode_card_coder_mini_lines()
}

/// Compact Mode Card Inspector Owl (20 cols x 24 height = 12 lines)
pub fn get_mode_card_inspector_lines() -> &'static [String] {
    get_mode_card_inspector_mini_lines()
}

/// Compact Mode Card Steward Owl (20 cols x 24 height = 12 lines)
pub fn get_mode_card_steward_lines() -> &'static [String] {
    get_mode_card_steward_mini_lines()
}

/// Compact Lifecycle Coder Owl (20 cols x 24 height = 12 lines)
pub fn get_lifecycle_coder_lines() -> &'static [String] {
    LIFECYCLE_CODER_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 20, 24))
        .as_slice()
}

/// Compact Lifecycle Inspector Owl (20 cols x 24 height = 12 lines)
pub fn get_lifecycle_inspector_lines() -> &'static [String] {
    LIFECYCLE_INSPECTOR_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 20, 24))
        .as_slice()
}

/// Compact Lifecycle Steward Owl (20 cols x 24 height = 12 lines)
pub fn get_lifecycle_steward_lines() -> &'static [String] {
    LIFECYCLE_STEWARD_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 20, 24))
        .as_slice()
}

/// Legacy Showcase Coder Owl
pub fn get_showcase_coder_lines() -> &'static [String] {
    get_showcase_coder_compact_lines()
}

/// Legacy Showcase Inspector Owl
pub fn get_showcase_inspector_lines() -> &'static [String] {
    get_showcase_inspector_compact_lines()
}

/// Legacy Showcase Steward Owl
pub fn get_showcase_steward_lines() -> &'static [String] {
    get_showcase_steward_compact_lines()
}

/// Legacy Wide Showcase Coder Owl
pub fn get_showcase_coder_wide_lines() -> &'static [String] {
    get_showcase_coder_compact_lines()
}

/// Legacy Wide Showcase Inspector Owl
pub fn get_showcase_inspector_wide_lines() -> &'static [String] {
    get_showcase_inspector_compact_lines()
}

/// Legacy Wide Showcase Steward Owl
pub fn get_showcase_steward_wide_lines() -> &'static [String] {
    get_showcase_steward_compact_lines()
}

/// Ultra-sharp High-definition Coder Owl
pub fn get_hd_coder_lines() -> &'static [String] {
    HD_CODER_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 90, 108))
        .as_slice()
}

/// Ultra-sharp High-definition Inspector Owl
pub fn get_hd_inspector_lines() -> &'static [String] {
    HD_INSPECTOR_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 89, 108))
        .as_slice()
}

/// Ultra-sharp High-definition Steward Owl
pub fn get_hd_steward_lines() -> &'static [String] {
    HD_STEWARD_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 89, 108))
        .as_slice()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskLifecycleState {
    /// Task idle: Custos standing by for user instructions
    Idle,
    /// Repository analysis, verification, checking evidence, or waiting for approval
    InspectionApproval,
    /// Task is running, provider is coding or executing
    RunningCoding,
    /// Task completed successfully
    Succeeded,
}

/// Display the integrated asset card corresponding to the task lifecycle state
/// Dynamically scales between side-by-side on wide screens and clean stacked on narrow screens.
pub fn print_task_lifecycle_card(state: TaskLifecycleState, details: &str) {
    let (tag, color_code, lines) = match state {
        TaskLifecycleState::Idle => (
            "TASK IDLE",
            "\x1b[38;2;125;211;252;1m",
            get_lifecycle_coder_lines(),
        ),
        TaskLifecycleState::InspectionApproval => (
            "INSPECTION & VERIFICATION",
            "\x1b[38;2;251;191;36;1m",
            get_lifecycle_inspector_lines(),
        ),
        TaskLifecycleState::RunningCoding => (
            "TASK RUNNING — CODING & EXECUTION",
            "\x1b[38;2;56;189;248;1m",
            get_lifecycle_coder_lines(),
        ),
        TaskLifecycleState::Succeeded => (
            "TASK COMPLETED SUCCESSFULLY",
            "\x1b[38;2;52;211;153;1m",
            get_lifecycle_steward_lines(),
        ),
    };

    let (_, term_cols) = console::Term::stdout().size();

    println!();
    if term_cols >= 82 {
        let box_w = (term_cols as usize).saturating_sub(26).clamp(50, 84);
        let top_border = format!("  \x1b[38;2;71;85;105m╭{}╮\x1b[0m", "─".repeat(box_w.saturating_sub(2)));
        let bot_border = format!("  \x1b[38;2;71;85;105m╰{}╯\x1b[0m", "─".repeat(box_w.saturating_sub(2)));
        let empty_line = format!("  \x1b[38;2;71;85;105m│{}│\x1b[0m", " ".repeat(box_w.saturating_sub(2)));

        let format_box_line = |content: &str| -> String {
            let vis_len = console::measure_text_width(content);
            let pad = box_w.saturating_sub(vis_len + 4);
            format!("  \x1b[38;2;71;85;105m│\x1b[0m {} {:pad$}\x1b[38;2;71;85;105m│\x1b[0m", content, "", pad = pad)
        };

        let tag_formatted = format!("{}{}\x1b[0m", color_code, tag);
        let details_formatted = format!("\x1b[38;2;226;232;240m{}\x1b[0m", details);

        let box_lines = [
            top_border,
            format_box_line(&tag_formatted),
            empty_line.clone(),
            format_box_line(&details_formatted),
            empty_line.clone(),
            format_box_line("\x1b[38;2;148;163;184mCustos Runtime Engine • Autonomous Sandbox Worktree\x1b[0m"),
            empty_line,
            bot_border,
        ];

        let max_rows = lines.len().max(box_lines.len());
        let blank_owl = "                    "; // 20 spaces
        for i in 0..max_rows {
            let left = if i < lines.len() { &lines[i] } else { blank_owl };
            let right = if i < box_lines.len() { &box_lines[i] } else { "" };
            println!("  {}  {}", left, right);
        }
    } else {
        println!("  {}[ {} ]\x1b[0m", color_code, tag);
        if !details.is_empty() {
            println!("  \x1b[38;2;226;232;240m{}\x1b[0m", details);
        }
        println!("  \x1b[38;2;71;85;105m────────────────────────────────────────────────────────────\x1b[0m");
        for line in lines {
            println!("  {}", line);
        }
    }
    println!();
}

/// When a task fails, display a text-only error report.
/// Absolutely no image modification or creation as mandated by specifications.
pub fn print_task_failure_report(task_id: &str, error_message: &str, rationale: Option<&str>) {
    println!();
    println!("  \x1b[38;2;239;68;68;1m╔════════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("  \x1b[38;2;239;68;68;1m║                     ✖ TASK FAILED (TEXT REPORT)                    ║\x1b[0m");
    println!("  \x1b[38;2;239;68;68;1m╚════════════════════════════════════════════════════════════════════╝\x1b[0m");
    println!("  \x1b[38;2;248;113;113;1mTask ID:\x1b[0m   {}", task_id);
    println!("  \x1b[38;2;248;113;113;1mStatus:\x1b[0m    \x1b[38;2;239;68;68;1mFAILED\x1b[0m");
    println!("  \x1b[38;2;248;113;113;1mError:\x1b[0m     {}", error_message);
    if let Some(r) = rationale {
        println!("  \x1b[38;2;248;113;113;1mRationale:\x1b[0m {}", r);
    }
    println!("  \x1b[38;2;148;163;184mNote: Asset intact. No image created or altered for failed state.\x1b[0m");
    println!("  \x1b[38;2;71;85;105m────────────────────────────────────────────────────────────────────\x1b[0m");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_loading_all() {
        for asset in &[
            AssetKind::Mascot,
            AssetKind::Coder,
            AssetKind::Inspector,
            AssetKind::Steward,
        ] {
            let img = load_asset_image(*asset);
            assert!(img.is_ok(), "Failed to load asset: {:?}", asset);
            let img = img.unwrap();
            let trans = make_outer_background_transparent_and_bleed(&img);
            let cropped = autocrop_content(&trans);
            println!("Asset {:?}: raw={}x{}, cropped={}x{}", asset, img.width(), img.height(), cropped.width(), cropped.height());
            assert!(img.width() > 0);
            assert!(img.height() > 0);
        }
    }

    #[test]
    fn test_render_lines_dimensions() {
        let lines = render_asset_to_lines(AssetKind::Mascot, 44, 52);
        assert_eq!(lines.len(), 26);
        for line in lines {
            assert!(!line.is_empty());
        }

        let hd_lines = render_asset_to_lines(AssetKind::Coder, 90, 108);
        assert_eq!(hd_lines.len(), 54);
        for line in hd_lines {
            assert!(!line.is_empty());
        }

        let mini_coder = get_showcase_coder_mini_lines();
        assert_eq!(mini_coder.len(), 9);

        let mini_inspector = get_showcase_inspector_mini_lines();
        assert_eq!(mini_inspector.len(), 9);

        let mini_steward = get_showcase_steward_mini_lines();
        assert_eq!(mini_steward.len(), 9);

        let compact_coder = get_showcase_coder_compact_lines();
        assert_eq!(compact_coder.len(), 12);

        let compact_inspector = get_showcase_inspector_compact_lines();
        assert_eq!(compact_inspector.len(), 12);

        let compact_steward = get_showcase_steward_compact_lines();
        assert_eq!(compact_steward.len(), 12);

        let mode_card_coder_mini = get_mode_card_coder_mini_lines();
        assert_eq!(mode_card_coder_mini.len(), 12);

        let mode_card_coder = get_mode_card_coder_lines();
        assert_eq!(mode_card_coder.len(), 12);
    }
}
