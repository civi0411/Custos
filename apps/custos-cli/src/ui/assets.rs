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
    Mascot,

    Coder,

    Inspector,

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

pub fn load_asset_image(asset: AssetKind) -> Result<DynamicImage, image::ImageError> {
    image::load_from_memory(asset.raw_bytes())
}

fn make_outer_background_transparent_and_bleed(
    img: &DynamicImage,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return rgba;
    }

    let mut visited = vec![false; (w * h) as usize];
    let mut q = VecDeque::new();

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

    let original = rgba.clone();
    for (idx, &is_outer_bg) in visited.iter().enumerate() {
        let ux = (idx as u32) % w;
        let uy = (idx as u32) / w;
        if is_outer_bg {
            let mut nearest_fg = None;
            for (dx, dy) in &[
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-2, 0),
                (2, 0),
                (0, -2),
                (0, 2),
            ] {
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
                fg_col[3] = 0;
                rgba.put_pixel(ux, uy, fg_col);
            } else {
                rgba.put_pixel(ux, uy, Rgba([0, 0, 0, 0]));
            }
        }
    }

    rgba
}

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

fn downsample_pixel_art_features(
    cropped: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    target_w: u32,
    target_h: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (src_w, src_h) = cropped.dimensions();
    let mut out = ImageBuffer::new(target_w, target_h);
    if src_w == 0 || src_h == 0 || target_w == 0 || target_h == 0 {
        return out;
    }

    for ty in 0..target_h {
        let y0 = ((ty as u64 * src_h as u64) / target_h as u64) as u32;
        let y1 = (((ty as u64 + 1) * src_h as u64) / target_h as u64)
            .min(src_h as u64)
            .max((y0 + 1) as u64) as u32;

        for tx in 0..target_w {
            let x0 = ((tx as u64 * src_w as u64) / target_w as u64) as u32;
            let x1 = (((tx as u64 + 1) * src_w as u64) / target_w as u64)
                .min(src_w as u64)
                .max((x0 + 1) as u64) as u32;

            let total_pixels = (x1 - x0) * (y1 - y0);
            if total_pixels == 0 {
                continue;
            }

            let mut vis_count = 0u32;
            let mut dark_count = 0u32;
            let mut vivid_count = 0u32;

            let mut sum_r = 0.0f32;
            let mut sum_g = 0.0f32;
            let mut sum_b = 0.0f32;

            let mut dark_r = 0.0f32;
            let mut dark_g = 0.0f32;
            let mut dark_b = 0.0f32;

            let mut vivid_r = 0.0f32;
            let mut vivid_g = 0.0f32;
            let mut vivid_b = 0.0f32;

            for y in y0..y1 {
                for x in x0..x1 {
                    let p = cropped.get_pixel(x, y);
                    if p[3] <= 40 {
                        continue;
                    }
                    vis_count += 1;
                    let r = p[0] as f32;
                    let g = p[1] as f32;
                    let b = p[2] as f32;

                    sum_r += r;
                    sum_g += g;
                    sum_b += b;

                    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                    let max_c = r.max(g).max(b);
                    let min_c = r.min(g).min(b);
                    let range = max_c - min_c;

                    if range > 45.0 && luma > 40.0 && luma < 248.0 {
                        vivid_count += 1;
                        vivid_r += r;
                        vivid_g += g;
                        vivid_b += b;
                    }

                    if luma < 58.0 {
                        dark_count += 1;
                        dark_r += r;
                        dark_g += g;
                        dark_b += b;
                    }
                }
            }

            if vis_count * 100 < total_pixels * 35 {
                out.put_pixel(tx, ty, Rgba([0, 0, 0, 0]));
                continue;
            }

            let vivid_ratio = vivid_count as f32 / vis_count as f32;
            let dark_ratio = dark_count as f32 / vis_count as f32;

            if vivid_ratio >= 0.08 {
                let vr = vivid_r / vivid_count as f32;
                let vg = vivid_g / vivid_count as f32;
                let vb = vivid_b / vivid_count as f32;
                let vluma = 0.299 * vr + 0.587 * vg + 0.114 * vb;

                let br = (vluma + (vr - vluma) * 1.40).clamp(0.0, 255.0) as u8;
                let bg = (vluma + (vg - vluma) * 1.40).clamp(0.0, 255.0) as u8;
                let bb = (vluma + (vb - vluma) * 1.40).clamp(0.0, 255.0) as u8;
                out.put_pixel(tx, ty, Rgba([br, bg, bb, 255]));
            } else if dark_ratio >= 0.12 {
                let dr = (dark_r / dark_count as f32).clamp(0.0, 255.0) as u8;
                let dg = (dark_g / dark_count as f32).clamp(0.0, 255.0) as u8;
                let db = (dark_b / dark_count as f32).clamp(0.0, 255.0) as u8;
                out.put_pixel(tx, ty, Rgba([dr, dg, db, 255]));
            } else {
                let ar = sum_r / vis_count as f32;
                let ag = sum_g / vis_count as f32;
                let ab = sum_b / vis_count as f32;
                let aluma = 0.299 * ar + 0.587 * ag + 0.114 * ab;

                if aluma > 185.0 {
                    out.put_pixel(tx, ty, Rgba([248, 250, 254, 255]));
                } else if aluma < 90.0 {
                    out.put_pixel(
                        tx,
                        ty,
                        Rgba([(ar * 0.85) as u8, (ag * 0.85) as u8, (ab * 0.85) as u8, 255]),
                    );
                } else {
                    out.put_pixel(tx, ty, Rgba([ar as u8, ag as u8, ab as u8, 255]));
                }
            }
        }
    }

    out
}

pub fn render_asset_to_lines(asset: AssetKind, target_w: u32, target_h: u32) -> Vec<String> {
    render_pixel_art_to_lines(asset, target_w, target_h)
}

pub fn render_pixel_art_to_lines(asset: AssetKind, target_w: u32, target_h: u32) -> Vec<String> {
    let img = match load_asset_image(asset) {
        Ok(i) => i,
        Err(_) => return Vec::new(),
    };
    let trans_img = make_outer_background_transparent_and_bleed(&img);
    let cropped = autocrop_content(&trans_img);
    let downsampled = downsample_pixel_art_features(&cropped, target_w, target_h);
    rasterize_to_half_blocks(&downsampled, target_w, target_h)
}

fn rasterize_to_half_blocks(
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    target_w: u32,
    target_h: u32,
) -> Vec<String> {
    let mut lines = Vec::with_capacity((target_h as usize + 1) / 2);

    for row in (0..target_h).step_by(2) {
        let mut line = String::with_capacity(target_w as usize * 32);

        for col in 0..target_w {
            let p1 = img.get_pixel(col, row);
            let p2 = if row + 1 < target_h {
                img.get_pixel(col, row + 1)
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
                    let _ = write!(line, "\x1b[38;2;{};{};{}m▄\x1b[0m", p2[0], p2[1], p2[2]);
                }
                (true, false) => {
                    let _ = write!(line, "\x1b[38;2;{};{};{}m▀\x1b[0m", p1[0], p1[1], p1[2]);
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

static MASCOT_BANNER_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static SHOWCASE_CODER_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_INSPECTOR_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_STEWARD_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static SHOWCASE_CODER_MED_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_INSPECTOR_MED_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_STEWARD_MED_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static SHOWCASE_CODER_COMPACT_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_INSPECTOR_COMPACT_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_STEWARD_COMPACT_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static SHOWCASE_CODER_ULTRA_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_INSPECTOR_ULTRA_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static SHOWCASE_STEWARD_ULTRA_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static MODE_CARD_CODER_LARGE_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_INSPECTOR_LARGE_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_STEWARD_LARGE_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static MODE_CARD_CODER_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_INSPECTOR_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_STEWARD_MINI_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static MODE_CARD_CODER_SMALL_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_INSPECTOR_SMALL_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static MODE_CARD_STEWARD_SMALL_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static LIFECYCLE_CODER_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static LIFECYCLE_INSPECTOR_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static LIFECYCLE_STEWARD_CACHE: OnceLock<Vec<String>> = OnceLock::new();

static HD_CODER_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static HD_INSPECTOR_CACHE: OnceLock<Vec<String>> = OnceLock::new();
static HD_STEWARD_CACHE: OnceLock<Vec<String>> = OnceLock::new();

pub fn get_mascot_banner_lines() -> &'static [String] {
    MASCOT_BANNER_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Mascot, 52, 64))
        .as_slice()
}

pub fn get_showcase_coder_mini_lines() -> &'static [String] {
    SHOWCASE_CODER_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 22, 28))
        .as_slice()
}

pub fn get_showcase_inspector_mini_lines() -> &'static [String] {
    SHOWCASE_INSPECTOR_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 22, 28))
        .as_slice()
}

pub fn get_showcase_steward_mini_lines() -> &'static [String] {
    SHOWCASE_STEWARD_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 22, 28))
        .as_slice()
}

pub fn get_showcase_coder_med_lines() -> &'static [String] {
    SHOWCASE_CODER_MED_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 30, 38))
        .as_slice()
}

pub fn get_showcase_inspector_med_lines() -> &'static [String] {
    SHOWCASE_INSPECTOR_MED_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 30, 38))
        .as_slice()
}

pub fn get_showcase_steward_med_lines() -> &'static [String] {
    SHOWCASE_STEWARD_MED_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 30, 38))
        .as_slice()
}

pub fn get_showcase_coder_compact_lines() -> &'static [String] {
    SHOWCASE_CODER_COMPACT_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 40, 50))
        .as_slice()
}

pub fn get_showcase_inspector_compact_lines() -> &'static [String] {
    SHOWCASE_INSPECTOR_COMPACT_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 40, 50))
        .as_slice()
}

pub fn get_showcase_steward_compact_lines() -> &'static [String] {
    SHOWCASE_STEWARD_COMPACT_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 40, 50))
        .as_slice()
}

pub fn get_showcase_coder_ultra_lines() -> &'static [String] {
    SHOWCASE_CODER_ULTRA_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 48, 60))
        .as_slice()
}

pub fn get_showcase_inspector_ultra_lines() -> &'static [String] {
    SHOWCASE_INSPECTOR_ULTRA_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 48, 60))
        .as_slice()
}

pub fn get_showcase_steward_ultra_lines() -> &'static [String] {
    SHOWCASE_STEWARD_ULTRA_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 48, 60))
        .as_slice()
}

pub fn get_mode_card_coder_large_lines() -> &'static [String] {
    MODE_CARD_CODER_LARGE_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 32, 40))
        .as_slice()
}

pub fn get_mode_card_inspector_large_lines() -> &'static [String] {
    MODE_CARD_INSPECTOR_LARGE_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 32, 40))
        .as_slice()
}

pub fn get_mode_card_steward_large_lines() -> &'static [String] {
    MODE_CARD_STEWARD_LARGE_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 32, 40))
        .as_slice()
}

pub fn get_mode_card_coder_mini_lines() -> &'static [String] {
    MODE_CARD_CODER_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 26, 32))
        .as_slice()
}

pub fn get_mode_card_inspector_mini_lines() -> &'static [String] {
    MODE_CARD_INSPECTOR_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 26, 32))
        .as_slice()
}

pub fn get_mode_card_steward_mini_lines() -> &'static [String] {
    MODE_CARD_STEWARD_MINI_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 26, 32))
        .as_slice()
}

pub fn get_mode_card_coder_small_lines() -> &'static [String] {
    MODE_CARD_CODER_SMALL_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 22, 28))
        .as_slice()
}

pub fn get_mode_card_inspector_small_lines() -> &'static [String] {
    MODE_CARD_INSPECTOR_SMALL_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 22, 28))
        .as_slice()
}

pub fn get_mode_card_steward_small_lines() -> &'static [String] {
    MODE_CARD_STEWARD_SMALL_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 22, 28))
        .as_slice()
}

pub fn get_mode_card_coder_lines() -> &'static [String] {
    get_mode_card_coder_mini_lines()
}

pub fn get_mode_card_inspector_lines() -> &'static [String] {
    get_mode_card_inspector_mini_lines()
}

pub fn get_mode_card_steward_lines() -> &'static [String] {
    get_mode_card_steward_mini_lines()
}

pub fn get_lifecycle_coder_lines() -> &'static [String] {
    LIFECYCLE_CODER_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 28, 36))
        .as_slice()
}

pub fn get_lifecycle_inspector_lines() -> &'static [String] {
    LIFECYCLE_INSPECTOR_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 28, 36))
        .as_slice()
}

pub fn get_lifecycle_steward_lines() -> &'static [String] {
    LIFECYCLE_STEWARD_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 28, 36))
        .as_slice()
}

pub fn get_showcase_coder_lines() -> &'static [String] {
    get_showcase_coder_compact_lines()
}

pub fn get_showcase_inspector_lines() -> &'static [String] {
    get_showcase_inspector_compact_lines()
}

pub fn get_showcase_steward_lines() -> &'static [String] {
    get_showcase_steward_compact_lines()
}

pub fn get_showcase_coder_wide_lines() -> &'static [String] {
    get_showcase_coder_compact_lines()
}

pub fn get_showcase_inspector_wide_lines() -> &'static [String] {
    get_showcase_inspector_compact_lines()
}

pub fn get_showcase_steward_wide_lines() -> &'static [String] {
    get_showcase_steward_compact_lines()
}

pub fn get_hd_coder_lines() -> &'static [String] {
    HD_CODER_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Coder, 90, 108))
        .as_slice()
}

pub fn get_hd_inspector_lines() -> &'static [String] {
    HD_INSPECTOR_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Inspector, 89, 108))
        .as_slice()
}

pub fn get_hd_steward_lines() -> &'static [String] {
    HD_STEWARD_CACHE
        .get_or_init(|| render_asset_to_lines(AssetKind::Steward, 89, 108))
        .as_slice()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskLifecycleState {
    Idle,

    InspectionApproval,

    RunningCoding,

    Succeeded,
}

pub fn print_task_lifecycle_card(state: TaskLifecycleState, details: &str) {
    let (tag, color_code, asset) = match state {
        TaskLifecycleState::Idle => ("TASK IDLE", "\x1b[38;2;125;211;252;1m", AssetKind::Coder),
        TaskLifecycleState::InspectionApproval => (
            "INSPECTION & VERIFICATION",
            "\x1b[38;2;251;191;36;1m",
            AssetKind::Inspector,
        ),
        TaskLifecycleState::RunningCoding => (
            "TASK RUNNING — CODING & EXECUTION",
            "\x1b[38;2;56;189;248;1m",
            AssetKind::Coder,
        ),
        TaskLifecycleState::Succeeded => (
            "TASK COMPLETED SUCCESSFULLY",
            "\x1b[38;2;52;211;153;1m",
            AssetKind::Steward,
        ),
    };

    let term_w = super::get_terminal_width();

    println!();
    if term_w >= 75 {
        let owl_w_px = (((term_w / 5) & !1) as u32).clamp(18, 22);
        let owl_h_px = (((owl_w_px as f32 * 1.20 + 0.5) as u32) + 1) & !1;
        let lines = render_pixel_art_to_lines(asset, owl_w_px, owl_h_px);
        let owl_display_w = owl_w_px as usize;

        let box_w = (term_w.saturating_sub(owl_display_w + 6)).clamp(45, 100);
        let top_border = format!(
            "  \x1b[38;2;71;85;105m╭{}╮\x1b[0m",
            "─".repeat(box_w.saturating_sub(2))
        );
        let bot_border = format!(
            "  \x1b[38;2;71;85;105m╰{}╯\x1b[0m",
            "─".repeat(box_w.saturating_sub(2))
        );
        let empty_line = format!(
            "  \x1b[38;2;71;85;105m│{}│\x1b[0m",
            " ".repeat(box_w.saturating_sub(2))
        );

        let format_box_line = |content: &str| -> String {
            let vis_len = console::measure_text_width(content);
            let pad = box_w.saturating_sub(vis_len + 4);
            format!(
                "  \x1b[38;2;71;85;105m│\x1b[0m {} {:pad$}\x1b[38;2;71;85;105m│\x1b[0m",
                content,
                "",
                pad = pad
            )
        };

        let tag_formatted = format!("{}{}\x1b[0m", color_code, tag);
        let details_formatted = format!("\x1b[38;2;226;232;240m{}\x1b[0m", details);

        let box_lines = [
            top_border,
            format_box_line(&tag_formatted),
            empty_line.clone(),
            format_box_line(&details_formatted),
            empty_line.clone(),
            format_box_line(
                "\x1b[38;2;148;163;184mCustos Runtime Engine • Autonomous Sandbox Worktree\x1b[0m",
            ),
            empty_line,
            bot_border,
        ];

        let max_rows = lines.len().max(box_lines.len());
        let blank_owl = " ".repeat(owl_display_w);
        for i in 0..max_rows {
            let left = if i < lines.len() {
                &lines[i]
            } else {
                &blank_owl
            };
            let right = if i < box_lines.len() {
                &box_lines[i]
            } else {
                ""
            };
            println!("  {}  {}", left, right);
        }
    } else {
        let owl_w_px = (((term_w / 3) & !1) as u32).clamp(16, 20);
        let owl_h_px = (((owl_w_px as f32 * 1.20 + 0.5) as u32) + 1) & !1;
        let lines = render_pixel_art_to_lines(asset, owl_w_px, owl_h_px);
        let owl_display_w = owl_w_px as usize;
        let pad_n = ((term_w.saturating_sub(owl_display_w)) / 2).min(10);
        let pad_str = " ".repeat(pad_n);
        let div_w = term_w.saturating_sub(4).clamp(24, 70);

        println!("  {}[ {} ]\x1b[0m", color_code, tag);
        if !details.is_empty() {
            println!("  \x1b[38;2;226;232;240m{}\x1b[0m", details);
        }
        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(div_w));
        for line in &lines {
            println!("{}{}", pad_str, line);
        }
        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(div_w));
        println!(
            "  \x1b[38;2;148;163;184mCustos Runtime Engine • Autonomous Sandbox Worktree\x1b[0m"
        );
    }
    println!();
}

pub fn print_task_failure_report(task_id: &str, error_message: &str, rationale: Option<&str>) {
    println!();
    println!("  \x1b[38;2;239;68;68;1m╔════════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("  \x1b[38;2;239;68;68;1m║                     ✖ TASK FAILED (TEXT REPORT)                    ║\x1b[0m");
    println!("  \x1b[38;2;239;68;68;1m╚════════════════════════════════════════════════════════════════════╝\x1b[0m");
    println!("  \x1b[38;2;248;113;113;1mTask ID:\x1b[0m   {}", task_id);
    println!("  \x1b[38;2;248;113;113;1mStatus:\x1b[0m    \x1b[38;2;239;68;68;1mFAILED\x1b[0m");
    println!(
        "  \x1b[38;2;248;113;113;1mError:\x1b[0m     {}",
        error_message
    );
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
            println!(
                "Asset {:?}: raw={}x{}, cropped={}x{}",
                asset,
                img.width(),
                img.height(),
                cropped.width(),
                cropped.height()
            );
            assert!(img.width() > 0);
            assert!(img.height() > 0);
        }
    }

    #[test]
    fn test_render_lines_dimensions() {
        let lines = render_asset_to_lines(AssetKind::Mascot, 52, 64);
        assert_eq!(lines.len(), 32);
        for line in &lines {
            assert!(!line.is_empty());
        }

        let hd_lines = render_asset_to_lines(AssetKind::Coder, 90, 108);
        assert_eq!(hd_lines.len(), 54);
        for line in &hd_lines {
            assert!(!line.is_empty());
        }

        let mini_coder = get_showcase_coder_mini_lines();
        assert_eq!(mini_coder.len(), 14);
        let mini_inspector = get_showcase_inspector_mini_lines();
        assert_eq!(mini_inspector.len(), 14);
        let mini_steward = get_showcase_steward_mini_lines();
        assert_eq!(mini_steward.len(), 14);

        let med_coder = get_showcase_coder_med_lines();
        assert_eq!(med_coder.len(), 19);

        let compact_coder = get_showcase_coder_compact_lines();
        assert_eq!(compact_coder.len(), 25);
        let compact_inspector = get_showcase_inspector_compact_lines();
        assert_eq!(compact_inspector.len(), 25);
        let compact_steward = get_showcase_steward_compact_lines();
        assert_eq!(compact_steward.len(), 25);

        let ultra_coder = get_showcase_coder_ultra_lines();
        assert_eq!(ultra_coder.len(), 30);

        let mode_card_coder_mini = get_mode_card_coder_mini_lines();
        assert_eq!(mode_card_coder_mini.len(), 16);
        let mode_card_coder = get_mode_card_coder_lines();
        assert_eq!(mode_card_coder.len(), 16);

        let mode_card_coder_large = get_mode_card_coder_large_lines();
        assert_eq!(mode_card_coder_large.len(), 20);

        let mode_card_coder_small = get_mode_card_coder_small_lines();
        assert_eq!(mode_card_coder_small.len(), 14);

        let lifecycle_coder = get_lifecycle_coder_lines();
        assert_eq!(lifecycle_coder.len(), 18);
    }
}
