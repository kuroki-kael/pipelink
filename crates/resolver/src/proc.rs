/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
};

use pipelink_common::types::{Pid, StartTime};

use crate::tokenize_cmdline;

/// Extracted process information from `/proc/[pid]`.
#[derive(Debug, Clone)]
pub(crate) struct ProcessInfo {
    pub pid: Pid,
    pub ppid: Option<Pid>,
    pub start_time: Option<StartTime>,
    pub comm: String,
    pub exe: Option<PathBuf>,
    pub cmdline: Vec<String>,
    pub environ: HashMap<String, String>,
    pub cgroup: Option<String>,
    pub cwd: Option<PathBuf>,
    pub root_dir: PathBuf,
}

impl ProcessInfo {
    /// Returns the binary file name (e.g., "spotify", "zen-bin").
    /// Falls back to `comm` or `cmdline[0]` if `/proc/[pid]/exe` is inaccessible.
    pub fn binary_name(&self) -> Option<&str> {
        if let Some(name) = self
            .exe
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
        {
            return Some(name);
        }
        if !self.comm.is_empty() {
            return Some(&self.comm);
        }
        self.cmdline
            .first()
            .and_then(|arg| Path::new(arg).file_name())
            .and_then(|a| a.to_str())
    }

    /// Checks if any command-line argument matches a given predicate.
    #[allow(dead_code)]
    pub fn has_argument_matching<F>(&self, predicate: F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        self.cmdline.iter().any(|arg| predicate(arg))
    }
}

/// Normalizes a path read from `/proc/[pid]/exe`.
///
/// When an application or package is updated on disk while running, the Linux
/// kernel appends `" (deleted)"` to the symlink target. This helper strips that
/// suffix so desktop databases and resolvers still find the correct binary!
#[must_use]
fn clean_proc_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(clean) = s.strip_suffix(" (deleted)") {
        PathBuf::from(clean)
    } else {
        path.to_path_buf()
    }
}

/// Reads process information for `pid` from the system `/proc`.
///
/// # Errors
/// Returns an [`io::Error`] if reading from `/proc` fails.
#[allow(dead_code)]
pub(crate) fn read_process_info(pid: Pid) -> io::Result<ProcessInfo> {
    read_process_info_from_proc(Path::new("/proc"), pid)
}

/// Reads process information for `pid` from a specified proc directory root.
///
/// # Errors
/// Returns an [`io::Error`] if reading from `proc_dir` fails.
pub(crate) fn read_process_info_from_proc(proc_dir: &Path, pid: Pid) -> io::Result<ProcessInfo> {
    let pid_dir = proc_dir.join(pid.to_string());
    if !pid_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Process {pid} not found in {}", proc_dir.display()),
        ));
    }

    // 1. Read /proc/[pid]/stat
    let stat_path = pid_dir.join("stat");
    let stat_str = fs::read_to_string(stat_path)?;
    let (comm, parent_pid, start_time) = parse_stat(pid, &stat_str);

    // 2. Read /proc/[pid]/comm (comm in stat is truncated to 16 chars)
    let comm = fs::read_to_string(pid_dir.join("comm")).map_or(comm, |s| s.trim().to_string());

    // 3. Read /proc/[pid]/exe and strip " (deleted)"
    let exe = fs::read_link(pid_dir.join("exe"))
        .ok()
        .map(|p| clean_proc_path(&p));

    // 4. Read /proc/[pid]/cmdline
    let cmdline = match fs::read(pid_dir.join("cmdline")) {
        Ok(bytes) => {
            let mut args: Vec<String> = bytes
                .split(|&b| b == 0)
                .filter(|slice| !slice.is_empty())
                .map(|slice| String::from_utf8_lossy(slice).trim().to_string())
                .collect();

            // Chromium / Electron sometimes rewrites argv[0] to include all
            // space-separated arguments
            if args.len() == 1 && (args[0].contains(" --") || args[0].contains(" /")) {
                let tokenized = tokenize_cmdline(&args[0]);
                if tokenized.len() > 1 {
                    args = tokenized;
                }
            }
            args
        },
        Err(_) => Vec::new(),
    };

    // 5. Read /proc/[pid]/environ
    let mut environ = HashMap::new();
    if let Ok(bytes) = fs::read(pid_dir.join("environ")) {
        for slice in bytes.split(|&b| b == 0) {
            if slice.is_empty() {
                continue;
            }
            let s = String::from_utf8_lossy(slice);
            if let Some((k, v)) = s.split_once('=') {
                environ.insert(k.to_string(), v.to_string());
            }
        }
    }

    // 6. Read /proc/[pid]/cgroup
    let cgroup = fs::read_to_string(pid_dir.join("cgroup")).ok();

    // 7. Read /proc/[pid]/cwd
    let cwd = fs::read_link(pid_dir.join("cwd")).ok();

    // 8. Root dir
    let root_dir = pid_dir.join("root");

    Ok(ProcessInfo {
        pid,
        ppid: parent_pid,
        start_time,
        comm,
        exe,
        cmdline,
        environ,
        cgroup,
        cwd,
        root_dir,
    })
}

/// Parses `/proc/[pid]/stat` to extract `comm`, `ppid`, and `start_time` (field 22).
fn parse_stat(_pid: Pid, stat_str: &str) -> (String, Option<Pid>, Option<StartTime>) {
    // Format: pid (comm) state ppid ... starttime
    let Some(open_paren) = stat_str.find('(') else {
        return (String::new(), None, None);
    };
    let Some(close_paren) = stat_str.rfind(')') else {
        return (String::new(), None, None);
    };

    if open_paren >= close_paren {
        return (String::new(), None, None);
    }

    let comm = stat_str[open_paren + 1..close_paren].to_string();
    let rest = stat_str[close_paren + 1..].trim();
    let fields: Vec<&str> = rest.split_whitespace().collect();

    let ppid = fields.get(1).and_then(|s| s.parse::<Pid>().ok());
    let start_time = fields.get(19).and_then(|s| s.parse::<StartTime>().ok());

    (comm, ppid, start_time)
}

/// Walks the process tree upwards from `pid` to collect its ancestry chain.
/// Returns `[info(pid), info(parent), info(grandparent), ...]`.
pub(crate) fn read_process_ancestry(
    proc_dir: &Path,
    pid: Pid,
    max_depth: usize,
) -> Vec<ProcessInfo> {
    let mut chain = Vec::new();
    let mut current_pid = pid;
    let mut visited = HashSet::new();

    while chain.len() < max_depth {
        if visited.contains(&current_pid) || current_pid == Pid(0) || current_pid == Pid(1) {
            break;
        }
        visited.insert(current_pid);

        match read_process_info_from_proc(proc_dir, current_pid) {
            Ok(info) => {
                let next_pid = info.ppid;
                chain.push(info);
                match next_pid {
                    Some(parent) if parent != current_pid && parent.0 > 1 => {
                        current_pid = parent;
                    },
                    _ => break,
                }
            },
            Err(_) => break,
        }
    }

    chain
}

/// Checks if a process with the given PID is currently alive in `proc_dir`.
#[must_use]
pub(crate) fn is_process_alive_in(proc_dir: &Path, pid: Pid) -> bool {
    proc_dir.join(pid.to_string()).exists()
}

/// Checks if a process with the given PID is currently alive on the system.
#[must_use]
pub fn is_process_alive(pid: Pid) -> bool {
    is_process_alive_in(Path::new("/proc"), pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_proc_path() {
        let p = Path::new("/usr/bin/spotify (deleted)");
        assert_eq!(clean_proc_path(p), PathBuf::from("/usr/bin/spotify"));

        let normal = Path::new("/usr/bin/firefox");
        assert_eq!(clean_proc_path(normal), PathBuf::from("/usr/bin/firefox"));
    }

    #[test]
    fn test_parse_stat() {
        let sample = "1234 (bash) S 1000 1234 1234 0 -1 4194304 300 0 0 0 10 5 0 0 20 0 1 0 54321 1234567 100";
        let (comm, ppid, start_time) = parse_stat(Pid(1234), sample);
        assert_eq!(comm, "bash");
        assert_eq!(ppid, Some(Pid(1000)));
        assert_eq!(start_time, Some(StartTime(54321)));
    }
}
