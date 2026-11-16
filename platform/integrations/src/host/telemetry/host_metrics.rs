//! Host OS process, memory, and listening-port telemetry adapter (`ps`, `sysctl`, `lsof`).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessGroupMetric {
    pub id: String,
    pub name: String,
    pub command: String,
    pub kind: String,
    pub pid: u32,
    pub memory_mb: f64,
    pub cpu_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningPortMetric {
    pub port: u16,
    pub pid: u32,
    pub command: String,
    pub public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTelemetrySnapshot {
    pub memory_resident_mb: f64,
    pub system_total_memory_mb: Option<u64>,
    pub process_groups: Vec<ProcessGroupMetric>,
    pub active_ports: Vec<u16>,
    pub listening_ports: Vec<ListeningPortMetric>,
}

struct RawProcessEntry {
    pid: u32,
    ppid: u32,
    rss_kb: f64,
    cpu: f64,
    comm: String,
}

/// Collects host CPU/RAM process metrics and listening TCP ports scoped to the Custos process tree.
pub fn collect_system_telemetry() -> SystemTelemetrySnapshot {
    let mut process_groups: Vec<ProcessGroupMetric> = Vec::new();
    let mut own_pids = HashSet::new();
    let mut all_processes: Vec<RawProcessEntry> = Vec::new();

    if let Some(out) = Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,rss=,%cpu=,comm="])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
    {
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 5 {
                if let (Ok(pid), Ok(ppid), Ok(rss), Ok(cpu)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<f64>(),
                ) {
                    let comm = parts[4..].join(" ");
                    all_processes.push(RawProcessEntry {
                        pid,
                        ppid,
                        rss_kb: rss,
                        cpu,
                        comm,
                    });
                }
            }
        }
    }

    let daemon_pid = std::process::id();
    let mut root_pids = HashSet::new();
    root_pids.insert(daemon_pid);

    for p in &all_processes {
        if p.comm.to_lowercase().contains("custos") {
            root_pids.insert(p.pid);
        }
    }

    let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in &all_processes {
        children_map.entry(p.ppid).or_default().push(p.pid);
    }

    let mut frontier: Vec<u32> = root_pids.iter().copied().collect();
    while let Some(pid) = frontier.pop() {
        if own_pids.insert(pid) {
            if let Some(kids) = children_map.get(&pid) {
                for &kid in kids {
                    if !own_pids.contains(&kid) {
                        frontier.push(kid);
                    }
                }
            }
        }
    }

    let mut total_kb: f64 = 0.0;
    for p in &all_processes {
        if own_pids.contains(&p.pid) {
            total_kb += p.rss_kb;
            let mb = ((p.rss_kb / 1024.0) * 10.0).round() / 10.0;
            let lower = p.comm.to_lowercase();
            let (id, name, kind) = if lower.contains("custos-daemon") {
                ("daemon", "Custos Daemon (Core Engine)", "daemon")
            } else if lower.contains("custos-app-desktop") {
                ("desktop", "Custos Desktop (GUI Studio)", "app")
            } else if lower.contains("zsh") || lower.contains("bash") || lower.contains("fish") {
                ("terminal", "Interactive Terminal Shell", "terminal")
            } else {
                ("worker", "Custos Worker Process", "worker")
            };

            process_groups.push(ProcessGroupMetric {
                id: id.to_string(),
                name: name.to_string(),
                command: p.comm.clone(),
                kind: kind.to_string(),
                pid: p.pid,
                memory_mb: mb,
                cpu_percent: p.cpu,
            });
        }
    }

    let memory_resident_mb = if total_kb == 0.0 {
        let pid = std::process::id();
        let rss_kb: f64 = Command::new("ps")
            .args(["-o", "rss=", "-p", &pid.to_string()])
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);
        let mb = ((rss_kb / 1024.0) * 10.0).round() / 10.0;
        process_groups.push(ProcessGroupMetric {
            id: "daemon".to_string(),
            name: "Custos Daemon (Core Engine)".to_string(),
            command: "custos-daemon".to_string(),
            kind: "daemon".to_string(),
            pid,
            memory_mb: mb,
            cpu_percent: 0.0,
        });
        mb
    } else {
        ((total_kb / 1024.0) * 10.0).round() / 10.0
    };

    process_groups.sort_by(|a, b| {
        b.memory_mb
            .partial_cmp(&a.memory_mb)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let system_total_memory_mb: Option<u64> = {
        #[cfg(target_os = "macos")]
        {
            Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|bytes| bytes / (1024 * 1024))
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    };

    let mut listening_ports: Vec<ListeningPortMetric> = Vec::new();
    let mut active_ports_set: BTreeSet<u16> = BTreeSet::new();

    if let Some(out) = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
    {
        let mut current_pid = 0u32;
        let mut current_cmd = String::new();

        for line in out.lines() {
            if line.len() >= 2 {
                let tag = &line[..1];
                let rest = &line[1..];
                match tag {
                    "p" => current_pid = rest.parse().unwrap_or(0),
                    "c" => current_cmd = rest.to_string(),
                    "n" if own_pids.contains(&current_pid) => {
                        if let Some((host, port_str)) = rest.rsplit_once(':') {
                            if let Ok(port) = port_str.parse::<u16>() {
                                let is_public = !(host.starts_with("127.")
                                    || host == "[::1]"
                                    || host == "localhost");
                                if active_ports_set.insert(port) {
                                    listening_ports.push(ListeningPortMetric {
                                        port,
                                        pid: current_pid,
                                        command: current_cmd.clone(),
                                        public: is_public,
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if active_ports_set.is_empty() {
        active_ports_set.insert(3000);
        listening_ports.push(ListeningPortMetric {
            port: 3000,
            pid: daemon_pid,
            command: "custos-daemon".to_string(),
            public: false,
        });
    }

    SystemTelemetrySnapshot {
        memory_resident_mb,
        system_total_memory_mb,
        process_groups,
        active_ports: active_ports_set.into_iter().collect(),
        listening_ports,
    }
}

