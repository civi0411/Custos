use console::style;

pub fn print_banner(version: &str) {
    super::art::print_main_owl_banner(version);
}

fn parse_semver(v: &str) -> Option<(u64, u64, u64, Option<&str>)> {
    let clean = v.trim().trim_start_matches('v');
    let mut parts_split = clean.splitn(2, '-');
    let num_part = parts_split.next()?;
    let pre = parts_split.next();

    let mut nums = Vec::new();
    for p in num_part.split('.') {
        nums.push(p.parse::<u64>().unwrap_or(0));
    }
    while nums.len() < 3 {
        nums.push(0);
    }
    Some((nums[0], nums[1], nums[2], pre))
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let (l_maj, l_min, l_pat, l_pre) = match parse_semver(latest) {
        Some(v) => v,
        None => return false,
    };
    let (c_maj, c_min, c_pat, c_pre) = match parse_semver(current) {
        Some(v) => v,
        None => return false,
    };

    if l_maj > c_maj {
        return true;
    }
    if l_maj < c_maj {
        return false;
    }
    if l_min > c_min {
        return true;
    }
    if l_min < c_min {
        return false;
    }
    if l_pat > c_pat {
        return true;
    }
    if l_pat < c_pat {
        return false;
    }

    if c_pre.is_some() && l_pre.is_none() {
        return true;
    }
    if let (Some(cp), Some(lp)) = (c_pre, l_pre) {
        return lp > cp;
    }
    false
}

pub fn get_available_update(current_version: &str) -> Option<String> {
    if let Ok(sim) = std::env::var("CUSTOS_SIMULATE_UPDATE") {
        let sim = sim.trim();
        if !sim.is_empty() && is_newer_version(sim, current_version) {
            return Some(sim.to_string());
        }
    }
    if let Ok(latest) = std::env::var("CUSTOS_LATEST_VERSION") {
        let latest = latest.trim();
        if !latest.is_empty() && is_newer_version(&latest, current_version) {
            return Some(latest.to_string());
        }
    }
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok()?;
    let path = std::path::PathBuf::from(home).join(".custos").join("update-check.json");
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(latest) = val.get("latestVersion").and_then(|v| v.as_str()) {
                if is_newer_version(latest, current_version) {
                    return Some(latest.to_string());
                }
            }
        }
    }
    None
}

use std::sync::atomic::{AtomicBool, Ordering};

static UPDATE_SHOWN: AtomicBool = AtomicBool::new(false);

pub fn check_and_print_update_notification(current_version: &str) {
    if std::env::var("CUSTOS_NO_UPDATE_NOTIFIER").is_ok() {
        return;
    }
    if UPDATE_SHOWN.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(latest) = get_available_update(current_version) {
        print_update_notification(current_version, &latest);
    }
}

#[allow(dead_code)]
pub fn print_text_banner(version: &str) {
    println!(
        "\x1b[38;2;125;211;252;1m   ______           __            \x1b[0m\n\
         \x1b[38;2;147;197;253;1m  / ____/_  _______/ /_____  _____\x1b[0m\n\
         \x1b[38;2;191;219;254;1m / /   / / / / ___/ __/ __ \\/ ___/\x1b[0m\n\
         \x1b[38;2;224;242;254;1m/ /___/ /_/ (__  ) /_/ /_/ (__  ) \x1b[0m\n\
         \x1b[38;2;240;248;255;1m\\____/\\__,_/____/\\__/\\____/____/  \x1b[0m"
    );
    println!(
        " {} {} {}",
        style("❄ Custos").bold().white(),
        style(format!("v{}", version)).dim(),
        style("— Guardian of Agentic Work").italic().dim()
    );
    println!(
        " {}\n",
        style("Human-governed runtime for specialized agentic workflows").dim()
    );
}

pub fn print_update_notification(current_version: &str, latest_version: &str) {
    let inner_width = 67;
    let line1 = format!(
        "   Update available {} {} {}",
        style(current_version).dim(),
        style("→").dim(),
        style(latest_version).green().bold()
    );
    let (os_name, update_cmd) = if cfg!(target_os = "windows") {
        ("Windows", "npm i -g custos-cli@latest")
    } else if cfg!(target_os = "macos") {
        ("macOS", "npm i -g custos-cli@latest")
    } else {
        ("Linux", "npm i -g custos-cli@latest")
    };
    let line2 = format!(
        "   To upgrade cleanly without keeping old version ({os_name}):"
    );
    let line3 = format!(
        "   Run {}",
        style(update_cmd).cyan().bold()
    );
    let line4 = format!(
        "   Changelog: {}",
        style("https://github.com/civi0411/Custos/releases").cyan()
    );

    let strip_ansi = |s: &str| -> String {
        let mut clean = String::new();
        let mut in_escape = false;
        for c in s.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if c == 'm' {
                    in_escape = false;
                }
            } else {
                clean.push(c);
            }
        }
        clean
    };

    let pad_line = |s: &str, width: usize| -> String {
        let visible_len = strip_ansi(s).chars().count();
        let padding = width.saturating_sub(visible_len);
        format!("{}{}", s, " ".repeat(padding))
    };

    let top = format!("\x1b[33m╭{}╮\x1b[0m", "─".repeat(inner_width));
    let empty = format!("\x1b[33m│\x1b[0m{}\x1b[33m│\x1b[0m", " ".repeat(inner_width));
    let bot = format!("\x1b[33m╰{}╯\x1b[0m", "─".repeat(inner_width));

    println!();
    println!("{}", top);
    println!("{}", empty);
    println!("\x1b[33m│\x1b[0m{}\x1b[33m│\x1b[0m", pad_line(&line1, inner_width));
    println!("\x1b[33m│\x1b[0m{}\x1b[33m│\x1b[0m", pad_line(&line2, inner_width));
    println!("\x1b[33m│\x1b[0m{}\x1b[33m│\x1b[0m", pad_line(&line3, inner_width));
    println!("{}", empty);
    println!("\x1b[33m│\x1b[0m{}\x1b[33m│\x1b[0m", pad_line(&line4, inner_width));
    println!("{}", empty);
    println!("{}", bot);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_update_notification_executes() {
        print_update_notification("0.1.0-alpha", "0.2.0");
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.2.0", "0.1.0-alpha"));
        assert!(is_newer_version("0.1.0", "0.1.0-alpha"));
        assert!(!is_newer_version("0.1.0-alpha", "0.1.0-alpha"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
    }
}
