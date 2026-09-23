//! Mascot Asset Display for Custos CLI
//!
//! Direct rendering of integrated image assets (`owl.png`, `custos-owl-coder.png`,
//! `custos-owl-inspector.png`, `custos-owl-steward.png`).
//! Preserves the original ANSI Shadow Custos banner wordmark aligned with the mascot asset.
//! Fully dynamic layout adapting smoothly from compact split windows to wide full-screen displays.

use crate::ui::OperationalMode;

/// Helper to center-pad text within a given column width
fn center_text(text: &str, width: usize) -> String {
    let vis = console::measure_text_width(text);
    if vis >= width {
        return text.to_string();
    }
    let left_pad = (width - vis) / 2;
    let right_pad = width - vis - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}

/// Render the project mascot (owl.png / owl.jpg) standing beside the ANSI Shadow Custos banner.
/// Dynamically responsive:
/// - Wide/Full-screen (>= 110 cols): Side-by-side mascot (44 cols) + banner (~60 cols), center-aligned.
/// - Compact (< 110 cols): Stacks mascot centered above compact banner to prevent horizontal line wrapping.
pub fn print_main_owl_banner(version: &str) {
    let (_, term_cols) = console::Term::stdout().size();
    let term_w = if term_cols > 0 { term_cols as usize } else { 100 };

    let mascot_lines = super::assets::get_mascot_banner_lines();

    if term_w >= 110 {
        // Wide & Full-screen layout: Side-by-side mascot (44 cols) + ANSI shadow banner (~60 cols)
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
            String::new(),
            String::new(),
        ];

        // Dynamic center margin on large / full-screen displays
        let left_pad = if term_w >= 125 {
            ((term_w.saturating_sub(108)) / 2).min(18)
        } else {
            1
        };
        let pad_str = " ".repeat(left_pad);

        println!();
        let max_lines = mascot_lines.len().max(banner_lines.len());
        for i in 0..max_lines {
            let left = if i < mascot_lines.len() {
                &mascot_lines[i]
            } else {
                "                                                "
            };
            let right = if i < banner_lines.len() {
                &banner_lines[i]
            } else {
                ""
            };
            println!("{}{}{}{}", pad_str, left, "   ", right);
        }
        println!();
    } else {
        // Compact / Split Window layout (< 110 cols): Prevents line wrapping
        let mascot_pad = ((term_w.saturating_sub(44)) / 2).min(16);
        let pad_str = " ".repeat(mascot_pad);

        println!();
        for line in mascot_lines {
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
}

/// Print the 3 integrated asset owls side-by-side: Coder, Inspector, Steward.
/// Small, ultra-sharp, and fully dynamic across 3 responsive tiers:
/// 1. Full-screen / Wide (>= 120 cols): 20-col owls (12 lines tall), dynamic wide spacing & dividers.
/// 2. Medium (90..119 cols): 18-col owls (11 lines tall), balanced spacing & dividers.
/// 3. Compact (< 90 cols): 14-col owls (9 lines tall), compact spacing fitting down to 60 cols.
pub fn print_modes_showcase() {
    let (_, term_cols) = console::Term::stdout().size();
    let term_w = if term_cols > 0 { term_cols as usize } else { 100 };

    println!("  \x1b[38;2;125;211;252;1m❄ OPERATIONAL MODES \x1b[0m\x1b[38;2;148;163;184m(Chọn chế độ hoạt động)\x1b[0m");

    if term_w >= 120 {
        // TIER 1: Full-Screen / Wide
        // Owls are 20 cols wide x 12 lines tall (clean, small, ultra-sharp)
        let coder = super::assets::get_showcase_coder_compact_lines();
        let inspector = super::assets::get_showcase_inspector_compact_lines();
        let steward = super::assets::get_showcase_steward_compact_lines();
        let owl_w = 20;

        let divider_len = (term_w.saturating_sub(8)).clamp(84, 126);
        let available_space = divider_len.saturating_sub(owl_w * 3);
        let gap = (available_space / 4).clamp(6, 16);
        let left_margin = gap;

        let margin_str = " ".repeat(left_margin);
        let gap_str = " ".repeat(gap);
        let blank_owl = " ".repeat(owl_w);

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let max_h = coder.len().max(inspector.len()).max(steward.len());
        for i in 0..max_h {
            let c = if i < coder.len() { &coder[i] } else { &blank_owl };
            let r = if i < inspector.len() { &inspector[i] } else { &blank_owl };
            let a = if i < steward.len() { &steward[i] } else { &blank_owl };
            println!("  {}{}{}{}{}{}", margin_str, c, gap_str, r, gap_str, a);
        }

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let h1 = center_text("[ 1. Code ]", owl_w);
        let h2 = center_text("[ 2. Research ]", owl_w);
        let h3 = center_text("[ 3. Assitant ]", owl_w);
        println!(
            "  {}\x1b[38;2;125;211;252;1m{}\x1b[0m{}\x1b[38;2;251;191;36;1m{}\x1b[0m{}\x1b[38;2;52;211;153;1m{}\x1b[0m",
            margin_str, h1, gap_str, h2, gap_str, h3
        );

        let s1 = center_text("Lập trình & Mã nguồn", owl_w);
        let s2 = center_text("Điều tra & Kiểm thử", owl_w);
        let s3 = center_text("Trợ lý & Điều phối", owl_w);
        println!(
            "  {}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m\n",
            margin_str, s1, gap_str, s2, gap_str, s3
        );
    } else if term_w >= 90 {
        // TIER 2: Medium Window (90..119 cols)
        // Owls are 18 cols wide x 11 lines tall
        let coder = super::assets::get_showcase_coder_med_lines();
        let inspector = super::assets::get_showcase_inspector_med_lines();
        let steward = super::assets::get_showcase_steward_med_lines();
        let owl_w = 18;

        let divider_len = (term_w.saturating_sub(6)).min(86);
        let available_space = divider_len.saturating_sub(owl_w * 3);
        let gap = (available_space / 4).clamp(3, 7);
        let left_margin = gap;

        let margin_str = " ".repeat(left_margin);
        let gap_str = " ".repeat(gap);
        let blank_owl = " ".repeat(owl_w);

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let max_h = coder.len().max(inspector.len()).max(steward.len());
        for i in 0..max_h {
            let c = if i < coder.len() { &coder[i] } else { &blank_owl };
            let r = if i < inspector.len() { &inspector[i] } else { &blank_owl };
            let a = if i < steward.len() { &steward[i] } else { &blank_owl };
            println!("  {}{}{}{}{}{}", margin_str, c, gap_str, r, gap_str, a);
        }

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let h1 = center_text("[ 1. Code ]", owl_w);
        let h2 = center_text("[ 2. Research ]", owl_w);
        let h3 = center_text("[ 3. Assitant ]", owl_w);
        println!(
            "  {}\x1b[38;2;125;211;252;1m{}\x1b[0m{}\x1b[38;2;251;191;36;1m{}\x1b[0m{}\x1b[38;2;52;211;153;1m{}\x1b[0m",
            margin_str, h1, gap_str, h2, gap_str, h3
        );

        let s1 = center_text("Lập trình & Code", owl_w);
        let s2 = center_text("Kiểm tra & Audit", owl_w);
        let s3 = center_text("Trợ lý & Quản lý", owl_w);
        println!(
            "  {}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m\n",
            margin_str, s1, gap_str, s2, gap_str, s3
        );
    } else {
        // TIER 3: Compact / Split Window (< 90 cols)
        // Mini owls: 14 cols wide x 9 lines tall, gap = 2..3
        let coder = super::assets::get_showcase_coder_mini_lines();
        let inspector = super::assets::get_showcase_inspector_mini_lines();
        let steward = super::assets::get_showcase_steward_mini_lines();
        let owl_w = 14;

        let divider_len = (term_w.saturating_sub(4)).clamp(48, 70);
        let gap = 3;
        let left_margin = 1;

        let margin_str = " ".repeat(left_margin);
        let gap_str = " ".repeat(gap);
        let blank_owl = " ".repeat(owl_w);

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let max_h = coder.len().max(inspector.len()).max(steward.len());
        for i in 0..max_h {
            let c = if i < coder.len() { &coder[i] } else { &blank_owl };
            let r = if i < inspector.len() { &inspector[i] } else { &blank_owl };
            let a = if i < steward.len() { &steward[i] } else { &blank_owl };
            println!("  {}{}{}{}{}{}", margin_str, c, gap_str, r, gap_str, a);
        }

        println!("  \x1b[38;2;71;85;105m{}\x1b[0m", "─".repeat(divider_len));

        let h1 = center_text("[1. Code]", owl_w);
        let h2 = center_text("[2. Resrch]", owl_w);
        let h3 = center_text("[3. Assist]", owl_w);
        println!(
            "  {}\x1b[38;2;125;211;252;1m{}\x1b[0m{}\x1b[38;2;251;191;36;1m{}\x1b[0m{}\x1b[38;2;52;211;153;1m{}\x1b[0m",
            margin_str, h1, gap_str, h2, gap_str, h3
        );

        let s1 = center_text("Lập trình", owl_w);
        let s2 = center_text("Kiểm tra", owl_w);
        let s3 = center_text("Trợ lý", owl_w);
        println!(
            "  {}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m{}\x1b[38;2;203;213;225m{}\x1b[0m\n",
            margin_str, s1, gap_str, s2, gap_str, s3
        );
    }
}

/// Print the activated mode card with a small, crisp owl asset and a welcoming dialogue card.
/// Automatically adapts layout to terminal dimensions:
/// - Wide / Full-screen (>= 82 cols): Side-by-side with dialogue speech box stretching up to 92 cols.
/// - Compact (< 82 cols): Clean stacked card with zero horizontal wrapping.
pub fn print_mode_card(mode: OperationalMode) {
    let (tag, greet_title, greet_msg, bullet1, bullet2, bullet3, lines, color_code) = match mode {
        OperationalMode::Code => (
            "CHẾ ĐỘ: CODE MODE (LẬP TRÌNH & THỰC THI)",
            "Xin chào! Tôi là Custos Coder Owl. ❄",
            "Tôi có thể giúp gì cho bạn hôm nay?",
            "Lập trình tính năng, sửa lỗi & tái cấu trúc mã nguồn",
            "Thực thi an toàn trong Sandbox Worktree cách ly",
            "Tối ưu thuật toán & kiểm tra tính đúng đắn",
            super::assets::get_mode_card_coder_mini_lines(),
            "\x1b[38;2;125;211;252;1m",
        ),
        OperationalMode::Research => (
            "CHẾ ĐỘ: RESEARCH MODE (ĐIỀU TRA & NGHIÊN CỨU)",
            "Xin chào! Tôi là Custos Inspector Owl. ❄",
            "Tôi có thể giúp gì cho bạn hôm nay?",
            "Điều tra cấu trúc dự án & đối soát bằng chứng",
            "Phân tích tài liệu, kiến trúc & suy luận chuyên sâu",
            "Kiểm chứng các thay đổi trước khi xin phê duyệt",
            super::assets::get_mode_card_inspector_mini_lines(),
            "\x1b[38;2;251;191;36;1m",
        ),
        OperationalMode::Assitant => (
            "CHẾ ĐỘ: ASSITANT MODE (TRỢ LÝ & ĐIỀU PHỐI)",
            "Xin chào! Tôi là Custos Steward Owl. ❄",
            "Tôi có thể giúp gì cho bạn hôm nay?",
            "Điều phối quy trình tác vụ tự động theo chuẩn runtime",
            "Quản lý trạng thái Task, Epoch Lock & Audit Logs",
            "Theo dõi tiến trình & báo cáo kết quả thực thi",
            super::assets::get_mode_card_steward_mini_lines(),
            "\x1b[38;2;52;211;153;1m",
        ),
    };

    let (_, term_cols) = console::Term::stdout().size();
    let term_w = if term_cols > 0 { term_cols as usize } else { 100 };

    println!();
    if term_w >= 82 {
        // Dynamic dialogue box width: stretches gracefully up to 92 cols on full-screen / wide monitors
        let box_w = (term_w.saturating_sub(26)).clamp(56, 92);

        let format_line = |content: &str| -> String {
            let vis_len = console::measure_text_width(content);
            let pad = box_w.saturating_sub(vis_len + 4);
            format!(
                "  \x1b[38;2;71;85;105m│\x1b[0m {} {:pad$}\x1b[38;2;71;85;105m│\x1b[0m",
                content, "", pad = pad
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
        let b1 = format!("\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m", bullet1);
        let b2 = format!("\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m", bullet2);
        let b3 = format!("\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m", bullet3);
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
        let blank_owl = "                    "; // 20 spaces
        for i in 0..max_rows {
            let left = if i < lines.len() { &lines[i] } else { blank_owl };
            let right = if i < box_lines.len() { &box_lines[i] } else { "" };
            println!("  {}  {}", left, right);
        }
    } else {
        // Stacked layout for compact / narrow mobile / split windows
        println!("  {}[ {} ]\x1b[0m", color_code, tag);
        println!("  \x1b[38;2;248;250;252;1m{}\x1b[0m", greet_title);
        println!("  \x1b[38;2;125;211;252;1m\"{}\"\x1b[0m\n", greet_msg);
        for line in lines {
            println!("  {}", line);
        }
        println!();
        println!("  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m", bullet1);
        println!("  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m", bullet2);
        println!("  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m{}\x1b[0m", bullet3);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
