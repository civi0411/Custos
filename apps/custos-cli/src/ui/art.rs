use super::assets::{render_pixel_art_to_lines, AssetKind};
use crate::ui::{get_terminal_width, OperationalMode, ResponsiveTier};

fn center_text(text: &str, width: usize) -> String {
    let vis = console::measure_text_width(text);
    if vis >= width {
        return text.to_string();
    }
    let left_pad = (width - vis) / 2;
    let right_pad = width - vis - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}

fn compute_showcase_dims(term_w: usize) -> (u32, u32, usize, usize) {
    let min_margin = 2usize;
    let prefix = 2usize;
    let safe_term_w = term_w.saturating_sub(2).max(20);
    let overhead = prefix + 4 * min_margin;
    let raw_owl_w = safe_term_w.saturating_sub(overhead) / 3;

    let owl_w_px = ((raw_owl_w.min(22) & !1) as u32).max(6);
    let owl_h_px = (((owl_w_px as f32 * 1.20 + 0.5) as u32) + 1) & !1;
    let total_owl = owl_w_px as usize * 3;
    let remaining = safe_term_w.saturating_sub(prefix + total_owl);
    let gap = (remaining / 6).clamp(1, 10);
    let left_margin = (remaining.saturating_sub(gap * 2)) / 2;

    (owl_w_px, owl_h_px, gap, left_margin)
}

pub fn print_main_owl_banner(version: &str) {
    let term_w = get_terminal_width();
    let tier = ResponsiveTier::from_width(term_w);

    match tier {
        ResponsiveTier::UltraWide | ResponsiveTier::Wide => {
            let mascot_w_px = (((term_w * 2 / 5) & !1) as u32).clamp(36, 64);
            let mascot_h_px = (((mascot_w_px as f32 * 1.22 + 0.5) as u32) + 1) & !1;
            let mascot_lines =
                render_pixel_art_to_lines(AssetKind::Mascot, mascot_w_px, mascot_h_px);
            let mascot_w = mascot_w_px as usize;

            let banner_lines = [
                String::new(),
                String::new(),
                format!("\x1b[38;2;125;211;252;1m   ██████╗ ██╗   ██╗ ███████╗ ████████╗  ██████╗  ███████╗\x1b[0m"),
                format!("\x1b[38;2;147;197;253;1m  ██╔════╝ ██║   ██║ ██╔════╝ ╚══██╔══╝ ██╔═══██╗ ██╔════╝\x1b[0m"),
                format!("\x1b[38;2;186;230;253;1m  ██║      ██║   ██║ ███████╗    ██║    ██║   ██║ ███████╗\x1b[0m"),
                format!("\x1b[38;2;224;242;254;1m  ██║      ██║   ██║ ╚════██║    ██║    ██║   ██║ ╚════██║\x1b[0m"),
                format!("\x1b[38;2;240;248;255;1m  ╚██████╗ ╚██████╔╝ ███████║    ██║    ╚██████╔╝ ███████║\x1b[0m"),
                format!("\x1b[38;2;248;250;252;1m   ╚═════╝  ╚═════╝  ╚══════╝    ╚═╝     ╚═════╝  ╚══════╝\x1b[0m"),
                String::new(),
                format!("  \x1b[38;2;251;191;36;1m❄ Custos\x1b[0m \x1b[38;2;148;163;184mv{}\x1b[0m \x1b[38;2;125;211;252;3m— Guardian of Agentic Work\x1b[0m", version),
                format!("  \x1b[38;2;148;163;184mHuman-governed runtime for specialized agentic workflows\x1b[0m"),
                format!("  \x1b[38;2;51;65;85m────────────────────────────────────────────────────────────\x1b[0m"),
                format!("  \x1b[38;2;251;191;36m•\x1b[0m \x1b[38;2;248;250;252;1mIntelligence Engine:\x1b[0m \x1b[38;2;148;163;184mCustos Snowy Owl Mascot (owl.png)\x1b[0m"),
                format!("  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;248;250;252;1mConcurrency Defense:\x1b[0m \x1b[38;2;148;163;184mEpoch Optimistic Lock & Replay\x1b[0m"),
                format!("  \x1b[38;2;56;189;248m•\x1b[0m \x1b[38;2;248;250;252;1mWorktree Isolation:\x1b[0m  \x1b[38;2;148;163;184mSeatbelt & Bubblewrap Sandbox\x1b[0m"),
                format!("  \x1b[38;2;168;85;247m•\x1b[0m \x1b[38;2;248;250;252;1mHuman-in-the-Loop:\x1b[0m   \x1b[38;2;148;163;184mExplicit ExecutionPermit Grants\x1b[0m"),
                format!("  \x1b[38;2;52;211;153m•\x1b[0m \x1b[38;2;248;250;252;1mAudit Trail:\x1b[0m         \x1b[38;2;148;163;184mImmutable SQLite Task & Span Logs\x1b[0m"),
                String::new(),
                format!("  \x1b[38;2;51;65;85m────────────────────────────────────────────────────────────\x1b[0m"),
                format!("  \x1b[38;2;125;211;252mOperational Modes:\x1b[0m  \x1b[38;2;203;213;225m1. Code  │  2. Research  │  3. Assitant\x1b[0m"),
                format!("  \x1b[38;2;148;163;184mQuick Commands:\x1b[0m     \x1b[38;2;100;116;139mcustos vibe  │  status  │  advance  │  list\x1b[0m"),
                String::new(),
            ];

            let combined_w = mascot_w + 3 + 62;
            let left_pad_n = if term_w > combined_w {
                ((term_w - combined_w) / 2).min(24)
            } else {
                1
            };
            let pad_str = " ".repeat(left_pad_n);
            let blank_mascot = " ".repeat(mascot_w);

            println!();
            let max_lines = mascot_lines.len().max(banner_lines.len());
            for i in 0..max_lines {
                let left = if i < mascot_lines.len() {
                    &mascot_lines[i]
                } else {
                    &blank_mascot
                };
                let right = if i < banner_lines.len() {
                    &banner_lines[i]
                } else {
                    ""
                };
                println!("{}{}   {}", pad_str, left, right);
            }
            println!();
        }
        ResponsiveTier::Standard => {
            let mascot_w_px = (((term_w * 2 / 5) & !1) as u32).clamp(28, 44);
            let mascot_h_px = (((mascot_w_px as f32 * 1.22 + 0.5) as u32) + 1) & !1;
            let mascot_lines =
                render_pixel_art_to_lines(AssetKind::Mascot, mascot_w_px, mascot_h_px);
            let mascot_w = mascot_w_px as usize;
            let mascot_pad = ((term_w.saturating_sub(mascot_w)) / 2).min(20);
            let pad_str = " ".repeat(mascot_pad);

            println!();
            for line in &mascot_lines {
                println!("{}{}", pad_str, line);
            }
            println!();
            println!("  \x1b[38;2;251;191;36;1m❄ Custos\x1b[0m \x1b[38;2;148;163;184mv{}\x1b[0m \x1b[38;2;125;211;252;3m— Guardian of Agentic Work\x1b[0m", version);
            println!("  \x1b[38;2;148;163;184mHuman-governed runtime for specialized agentic workflows\x1b[0m");
            println!("  \x1b[38;2;51;65;85m────────────────────────────────────────────────────────────\x1b[0m");
            println!("  \x1b[38;2;251;191;36m•\x1b[0m \x1b[38;2;248;250;252;1mIntelligence Engine:\x1b[0m \x1b[38;2;148;163;184mCustos Snowy Owl Mascot\x1b[0m");
            println!("  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;248;250;252;1mConcurrency Defense:\x1b[0m \x1b[38;2;148;163;184mEpoch Optimistic Lock & Replay\x1b[0m");
            println!("  \x1b[38;2;56;189;248m•\x1b[0m \x1b[38;2;248;250;252;1mWorktree Isolation:\x1b[0m  \x1b[38;2;148;163;184mSeatbelt & Bubblewrap Sandbox\x1b[0m");
            println!("  \x1b[38;2;168;85;247m•\x1b[0m \x1b[38;2;248;250;252;1mHuman-in-the-Loop:\x1b[0m   \x1b[38;2;148;163;184mExplicit ExecutionPermit Grants\x1b[0m");
            println!("  \x1b[38;2;52;211;153m•\x1b[0m \x1b[38;2;248;250;252;1mAudit Trail:\x1b[0m         \x1b[38;2;148;163;184mImmutable SQLite Task & Span Logs\x1b[0m");
            println!("  \x1b[38;2;51;65;85m────────────────────────────────────────────────────────────\x1b[0m");
            println!("  \x1b[38;2;125;211;252mModes:\x1b[0m 1. Code │ 2. Research │ 3. Assitant\n");
        }
        ResponsiveTier::Compact => {
            let mascot_w_px = (((term_w * 2 / 5) & !1) as u32).clamp(18, 28);
            let mascot_h_px = (((mascot_w_px as f32 * 1.22 + 0.5) as u32) + 1) & !1;
            let mascot_lines =
                render_pixel_art_to_lines(AssetKind::Mascot, mascot_w_px, mascot_h_px);
            let mascot_w = mascot_w_px as usize;
            let mascot_pad = ((term_w.saturating_sub(mascot_w)) / 2).min(10);
            let pad_str = " ".repeat(mascot_pad);
            let div_w = term_w.saturating_sub(4).clamp(24, 60);

            println!();
            for line in &mascot_lines {
                println!("{}{}", pad_str, line);
            }
            println!();
            println!(
                "  \x1b[38;2;251;191;36;1m❄ Custos\x1b[0m \x1b[38;2;148;163;184mv{}\x1b[0m",
                version
            );
            println!("  \x1b[38;2;125;211;252;3m— Guardian of Agentic Work\x1b[0m");
            println!("  \x1b[38;2;51;65;85m{}\x1b[0m", "─".repeat(div_w));
            println!("  \x1b[38;2;125;211;252mModes:\x1b[0m 1. Code │ 2. Research │ 3. Assitant\n");
        }
    }
}

pub fn print_modes_showcase() {
    let term_w = get_terminal_width();

    println!("  \x1b[38;2;125;211;252;1m❄ OPERATIONAL MODES \x1b[0m\x1b[38;2;148;163;184m(Chọn chế độ hoạt động)\x1b[0m");

    if term_w >= 58 {
        let (owl_w_px, owl_h_px, gap, left_margin) = compute_showcase_dims(term_w);
        let owl_w = owl_w_px as usize;
        let total_owl = owl_w * 3;
        let divider_len = (total_owl + gap * 2 + left_margin * 2).min(term_w.saturating_sub(4));

        let margin_str = " ".repeat(left_margin);
        let gap_str = " ".repeat(gap);
        let blank = " ".repeat(owl_w);

        let coder = render_pixel_art_to_lines(AssetKind::Coder, owl_w_px, owl_h_px);
        let inspector = render_pixel_art_to_lines(AssetKind::Inspector, owl_w_px, owl_h_px);
        let steward = render_pixel_art_to_lines(AssetKind::Steward, owl_w_px, owl_h_px);

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let max_h = coder.len().max(inspector.len()).max(steward.len());
        for i in 0..max_h {
            let c = if i < coder.len() { &coder[i] } else { &blank };
            let r = if i < inspector.len() {
                &inspector[i]
            } else {
                &blank
            };
            let a = if i < steward.len() {
                &steward[i]
            } else {
                &blank
            };
            println!("  {}{}{}{}{}{}", margin_str, c, gap_str, r, gap_str, a);
        }

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let (h1_label, h2_label, h3_label) = if owl_w >= 16 {
            ("[ 1. Code ]", "[ 2. Research ]", "[ 3. Assitant ]")
        } else {
            ("[1. Code]", "[2. Resrch]", "[3. Assist]")
        };
        let (s1_label, s2_label, s3_label) = if owl_w >= 22 {
            (
                "Lập trình & Mã nguồn",
                "Điều tra & Kiểm thử",
                "Trợ lý & Điều phối",
            )
        } else if owl_w >= 16 {
            ("Lập trình & Code", "Kiểm tra & Audit", "Trợ lý & Quản lý")
        } else {
            ("Lập trình", "Kiểm tra", "Trợ lý")
        };

        let h1 = center_text(h1_label, owl_w);
        let h2 = center_text(h2_label, owl_w);
        let h3 = center_text(h3_label, owl_w);
        println!(
            "  {}\x1b[38;2;125;211;252;1m{}\x1b[0m{}\x1b[38;2;251;191;36;1m{}\x1b[0m{}\x1b[38;2;52;211;153;1m{}\x1b[0m",
            margin_str, h1, gap_str, h2, gap_str, h3
        );

        let s1 = center_text(s1_label, owl_w);
        let s2 = center_text(s2_label, owl_w);
        let s3 = center_text(s3_label, owl_w);
        println!(
            "  {}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m\n",
            margin_str, s1, gap_str, s2, gap_str, s3
        );
    } else {
        let div_w = term_w.saturating_sub(4).clamp(24, 54);
        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(div_w));
        println!("  \x1b[38;2;125;211;252;1m[ 1. Code ]\x1b[0m     \x1b[38;2;203;213;225mLập trình & Mã nguồn\x1b[0m");
        println!("  \x1b[38;2;251;191;36;1m[ 2. Research ]\x1b[0m \x1b[38;2;203;213;225mĐiều tra & Kiểm thử\x1b[0m");
        println!("  \x1b[38;2;52;211;153;1m[ 3. Assitant ]\x1b[0m \x1b[38;2;203;213;225mTrợ lý & Điều phối\x1b[0m");
        println!("  \x1b[38;2;71;85;105m{}\x1b[0m\n", "─".repeat(div_w));
    }
}

pub fn print_mode_card(mode: OperationalMode) {
    let (tag, greet_title, greet_msg, bullet1, bullet2, bullet3, color_code) = match mode {
        OperationalMode::Code => (
            "CHẾ ĐỘ: CODE MODE (LẬP TRÌNH & THỰC THI)",
            "Xin chào! Tôi là Custos Coder Owl. ❄",
            "Tôi có thể giúp gì cho bạn hôm nay?",
            "Lập trình tính năng, sửa lỗi & tái cấu trúc mã nguồn",
            "Thực thi an toàn trong Sandbox Worktree cách ly",
            "Tối ưu thuật toán & kiểm tra tính đúng đắn",
            "\x1b[38;2;125;211;252;1m",
        ),
        OperationalMode::Research => (
            "CHẾ ĐỘ: RESEARCH MODE (ĐIỀU TRA & NGHIÊN CỨU)",
            "Xin chào! Tôi là Custos Inspector Owl. ❄",
            "Tôi có thể giúp gì cho bạn hôm nay?",
            "Điều tra cấu trúc dự án & đối soát bằng chứng",
            "Phân tích tài liệu, kiến trúc & suy luận chuyên sâu",
            "Kiểm chứng các thay đổi trước khi xin phê duyệt",
            "\x1b[38;2;251;191;36;1m",
        ),
        OperationalMode::Assitant => (
            "CHẾ ĐỘ: ASSITANT MODE (TRỢ LÝ & ĐIỀU PHỐI)",
            "Xin chào! Tôi là Custos Steward Owl. ❄",
            "Tôi có thể giúp gì cho bạn hôm nay?",
            "Điều phối quy trình tác vụ tự động theo chuẩn runtime",
            "Quản lý trạng thái Task, Epoch Lock & Audit Logs",
            "Theo dõi tiến trình & báo cáo kết quả thực thi",
            "\x1b[38;2;52;211;153;1m",
        ),
    };

    let term_w = get_terminal_width();

    let asset = match mode {
        OperationalMode::Code => AssetKind::Coder,
        OperationalMode::Research => AssetKind::Inspector,
        OperationalMode::Assitant => AssetKind::Steward,
    };

    println!();

    if term_w >= 70 {
        let owl_w_px = 20u32;
        let owl_h_px = 24u32;
        let lines = render_pixel_art_to_lines(asset, owl_w_px, owl_h_px);

        let owl_display_w = owl_w_px as usize;
        let box_w = (term_w.saturating_sub(owl_display_w + 6)).clamp(38, 110);

        let format_line = |content: &str| -> String {
            let vis_len = console::measure_text_width(content);
            let pad = box_w.saturating_sub(vis_len + 4);
            format!(
                "  \x1b[38;2;71;85;105m│\x1b[0m {} {:pad$}\x1b[38;2;71;85;105m│\x1b[0m",
                content,
                "",
                pad = pad
            )
        };

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

        let tag_formatted = format!("{}{}\x1b[0m", color_code, tag);
        let title_formatted = format!("\x1b[38;2;248;250;252;1m{}\x1b[0m", greet_title);
        let msg_formatted = format!("\x1b[38;2;125;211;252;1m\"{}\"\x1b[0m", greet_msg);
        let b1 = format!(
            "\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m",
            bullet1
        );
        let b2 = format!(
            "\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m",
            bullet2
        );
        let b3 = format!(
            "\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m",
            bullet3
        );
        let hint = "\x1b[38;2;148;163;184mNhập mục tiêu hoặc yêu cầu nhiệm vụ bạn muốn thực hiện bên dưới.\x1b[0m";

        let box_lines = [
            top_border,
            format_line(&tag_formatted),
            empty_line.clone(),
            format_line(&title_formatted),
            format_line(&msg_formatted),
            empty_line.clone(),
            format_line(&b1),
            format_line(&b2),
            format_line(&b3),
            empty_line,
            format_line(hint),
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
        let div_w = term_w.saturating_sub(4).clamp(24, 66);

        println!("  {}[ {} ]\x1b[0m", color_code, tag);
        println!("  \x1b[38;2;248;250;252;1m{}\x1b[0m", greet_title);
        println!("  \x1b[38;2;125;211;252;1m\"{}\"\x1b[0m\n", greet_msg);
        for line in &lines {
            println!("{}{}", pad_str, line);
        }
        println!();
        println!("  \x1b[38;2;51;65;85m{}\x1b[0m", "─".repeat(div_w));
        println!(
            "  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m",
            bullet1
        );
        println!(
            "  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m",
            bullet2
        );
        println!(
            "  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m",
            bullet3
        );
        println!("  \x1b[38;2;51;65;85m{}\x1b[0m", "─".repeat(div_w));
        println!("  \x1b[38;2;148;163;184mNhập mục tiêu nhiệm vụ bên dưới:\x1b[0m");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ResponsiveTier;

    #[test]
    fn test_print_banner_executes() {
        print_main_owl_banner("0.1.0");
    }

    #[test]
    fn test_print_modes_showcase_executes() {
        print_modes_showcase();
    }

    #[test]
    fn test_print_mode_cards_executes() {
        print_mode_card(OperationalMode::Code);
        print_mode_card(OperationalMode::Research);
        print_mode_card(OperationalMode::Assitant);
    }

    #[test]
    fn test_print_mascot_sizes() {
        let version = "0.1.0";
        for (w, h) in [(12, 12), (14, 16), (16, 20)] {
            println!(
                "\n=== Banner test: Mascot w={} h={} ({} lines) ===",
                w,
                h,
                h / 2
            );
            let mascot_lines = render_pixel_art_to_lines(AssetKind::Mascot, w, h);
            let banner_lines = [
                format!("\x1b[38;2;125;211;252;1m   ██████╗ ██╗   ██╗ ███████╗ ████████╗  ██████╗  ███████╗\x1b[0m"),
                format!("\x1b[38;2;147;197;253;1m  ██╔════╝ ██║   ██║ ██╔════╝ ╚══██╔══╝ ██╔═══██╗ ██╔════╝\x1b[0m"),
                format!("\x1b[38;2;186;230;253;1m  ██║      ██║   ██║ ███████╗    ██║    ██║   ██║ ███████╗\x1b[0m"),
                format!("\x1b[38;2;224;242;254;1m  ██║      ██║   ██║ ╚════██║    ██║    ██║   ██║ ╚════██║\x1b[0m"),
                format!("\x1b[38;2;240;248;255;1m  ╚██████╗ ╚██████╔╝ ███████║    ██║    ╚██████╔╝ ███████║\x1b[0m"),
                format!("\x1b[38;2;248;250;252;1m   ╚═════╝  ╚═════╝  ╚══════╝    ╚═╝     ╚═════╝  ╚══════╝\x1b[0m"),
                String::new(),
                format!("  \x1b[38;2;251;191;36;1m❄ Custos\x1b[0m \x1b[38;2;148;163;184mv{}\x1b[0m \x1b[38;2;125;211;252;3m— Guardian of Agentic Work\x1b[0m", version),
                format!("  \x1b[38;2;148;163;184mHuman-governed runtime for specialized agentic workflows\x1b[0m"),
                format!("  \x1b[38;2;51;65;85m────────────────────────────────────────────────────────────\x1b[0m"),
            ];
            let max_l = mascot_lines.len().max(banner_lines.len());
            let blank = " ".repeat(w as usize);
            for i in 0..max_l {
                let m = if i < mascot_lines.len() {
                    &mascot_lines[i]
                } else {
                    &blank
                };
                let b = if i < banner_lines.len() {
                    &banner_lines[i]
                } else {
                    ""
                };
                println!("  {}   {}", m, b);
            }
        }
    }

    #[test]
    fn test_compute_showcase_dims_no_overflow() {
        for term_w in [
            40usize, 60, 75, 80, 90, 100, 120, 150, 180, 200, 220, 260, 300,
        ] {
            let (owl_w_px, _, gap, left_margin) = compute_showcase_dims(term_w);
            let owl_w = owl_w_px as usize;
            let total = 2 + left_margin + owl_w + gap + owl_w + gap + owl_w + left_margin;
            assert!(
                total <= term_w,
                "term_w={term_w}: total layout={total} exceeds terminal width"
            );
        }
    }

    #[test]
    fn test_responsive_tiers() {
        assert_eq!(ResponsiveTier::from_width(50), ResponsiveTier::Compact);
        assert_eq!(ResponsiveTier::from_width(80), ResponsiveTier::Standard);
        assert_eq!(ResponsiveTier::from_width(120), ResponsiveTier::Wide);
        assert_eq!(ResponsiveTier::from_width(180), ResponsiveTier::UltraWide);
    }
}
