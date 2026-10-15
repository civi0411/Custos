//! Apple Silicon Metal & Hardware Resource Prober
//!
//! Queries physical RAM, CPU brand string, real-time free memory, and
//! Metal unified memory allocation ceiling.

use custos_domain::LocalHardwareProfile;
use std::process::Command;

#[cfg(target_os = "macos")]
fn get_macos_free_ram() -> Option<u64> {
    let output = Command::new("vm_stat").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);

    let mut page_size: u64 = 16384; // Default on Apple Silicon
    if let Some(first_line) = text.lines().next() {
        if let Some(start) = first_line.find("page size of ") {
            let rest = &first_line[start + 13..];
            if let Some(end) = rest.find(" bytes") {
                if let Ok(ps) = rest[..end].trim().parse::<u64>() {
                    page_size = ps;
                }
            }
        }
    }

    let mut pages_free: u64 = 0;
    let mut pages_speculative: u64 = 0;
    let mut pages_inactive: u64 = 0;

    for line in text.lines() {
        if line.starts_with("Pages free:") {
            if let Some(num_str) = line.split(':').nth(1) {
                pages_free = num_str.trim().trim_end_matches('.').parse::<u64>().unwrap_or(0);
            }
        } else if line.starts_with("Pages speculative:") {
            if let Some(num_str) = line.split(':').nth(1) {
                pages_speculative = num_str.trim().trim_end_matches('.').parse::<u64>().unwrap_or(0);
            }
        } else if line.starts_with("Pages inactive:") {
            if let Some(num_str) = line.split(':').nth(1) {
                pages_inactive = num_str.trim().trim_end_matches('.').parse::<u64>().unwrap_or(0);
            }
        }
    }

    Some((pages_free + pages_speculative + pages_inactive) * page_size)
}

pub fn probe_local_hardware() -> LocalHardwareProfile {
    #[cfg(target_os = "macos")]
    {
        let is_apple_silicon = std::env::consts::ARCH == "aarch64";

        let chip_name = Command::new("sysctl")
            .arg("-n")
            .arg("machdep.cpu.brand_string")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if is_apple_silicon {
                    "Apple Silicon".to_string()
                } else {
                    "Generic Mac".to_string()
                }
            });

        let total_ram = Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(16 * 1024 * 1024 * 1024);

        // Metal VRAM ceiling (75% unified memory by default on macOS, or iogpu.wired_mem_limit)
        let wired_override = Command::new("sysctl")
            .arg("-n")
            .arg("iogpu.wired_mem_limit")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .filter(|&v| v > 0);

        let metal_vram = wired_override.unwrap_or_else(|| (total_ram * 3) / 4);
        let free_ram = get_macos_free_ram().unwrap_or(total_ram / 4);

        LocalHardwareProfile {
            total_ram_bytes: total_ram,
            metal_vram_limit_bytes: metal_vram,
            free_ram_bytes: free_ram,
            is_apple_silicon,
            chip_name,
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        LocalHardwareProfile {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            metal_vram_limit_bytes: 8 * 1024 * 1024 * 1024,
            free_ram_bytes: 4 * 1024 * 1024 * 1024,
            is_apple_silicon: false,
            chip_name: "Generic Host".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_local_hardware() {
        let hw = probe_local_hardware();
        assert!(hw.total_ram_bytes > 0);
        assert!(hw.metal_vram_limit_bytes > 0);
        assert!(hw.metal_vram_limit_bytes <= hw.total_ram_bytes);
        println!("Probed Hardware: {:?}", hw);
    }
}
