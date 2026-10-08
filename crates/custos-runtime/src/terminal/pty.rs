//! Native POSIX PTY Process Management
//!
//! Spawns real pseudo-terminal processes on Unix (macOS / Linux) with:
//! - Master / slave pair creation via `libc::openpty`
//! - Controlling terminal setup and standard IO redirection
//! - Dynamic window resizing via `TIOCSWINSZ`
//! - Raw non-blocking / streaming I/O with process lifecycle observation

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// Native PTY handle managing a spawned child process and its master PTY descriptor.
pub struct PtyHandle {
    #[cfg(unix)]
    master_fd: i32,
    pid: u32,
    terminated: Arc<AtomicBool>,
}

impl PtyHandle {
    /// Spawns a new shell or command inside a dedicated pseudo-terminal.
    pub fn spawn(
        working_dir: &Path,
        command_override: Option<&str>,
        cols: u16,
        rows: u16,
        output_tx: mpsc::Sender<Vec<u8>>,
    ) -> io::Result<(Self, tokio::sync::oneshot::Receiver<Option<i32>>)> {
        #[cfg(unix)]
        {
            let mut master: libc::c_int = -1;
            let mut slave: libc::c_int = -1;

            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };

            let ret = unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &ws as *const libc::winsize as *mut _,
                )
            };

            if ret != 0 {
                return Err(io::Error::last_os_error());
            }

            // Determine shell binary
            let default_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            let (prog, args) = match command_override {
                Some(cmd) if !cmd.trim().is_empty() => {
                    (default_shell.clone(), vec!["-c".to_string(), cmd.to_string()])
                }
                _ => (default_shell, vec!["-l".to_string()]),
            };

            let mut cmd = std::process::Command::new(&prog);
            cmd.args(&args);
            cmd.current_dir(working_dir);
            cmd.env("TERM", "xterm-256color");
            cmd.env("COLORTERM", "truecolor");
            cmd.env("PAGER", "cat");

            unsafe {
                cmd.pre_exec(move || {
                    if libc::setsid() < 0 {
                        return Err(io::Error::last_os_error());
                    }
                    #[cfg(any(target_os = "macos", target_os = "linux"))]
                    {
                        libc::ioctl(slave, libc::TIOCSCTTY as _, 0);
                    }
                    if libc::dup2(slave, 0) < 0
                        || libc::dup2(slave, 1) < 0
                        || libc::dup2(slave, 2) < 0
                    {
                        return Err(io::Error::last_os_error());
                    }
                    if slave > 2 {
                        libc::close(slave);
                    }
                    Ok(())
                });
            }

            let mut child = match cmd.spawn() {
                Ok(c) => {
                    unsafe {
                        libc::close(slave);
                    }
                    c
                }
                Err(e) => {
                    unsafe {
                        libc::close(slave);
                        libc::close(master);
                    }
                    return Err(e);
                }
            };

            let pid = child.id();
            let terminated = Arc::new(AtomicBool::new(false));
            let term_flag = terminated.clone();

            let (exit_tx, exit_rx) = tokio::sync::oneshot::channel();

            // Background reader thread reading raw PTY stream
            let master_reader = master;
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    let n = unsafe {
                        libc::read(
                            master_reader,
                            buf.as_mut_ptr() as *mut libc::c_void,
                            buf.len(),
                        )
                    };

                    if n > 0 {
                        let chunk = buf[..n as usize].to_vec();
                        if output_tx.blocking_send(chunk).is_err() {
                            break;
                        }
                    } else if n == 0 {
                        // Clean EOF
                        break;
                    } else {
                        // Error or EIO (child process exited and closed slave)
                        break;
                    }
                }
                debug!(pid = pid, "PTY reader loop finished");
            });

            // Background waiter thread observing child process exit
            std::thread::spawn(move || {
                let status = child.wait().ok();
                term_flag.store(true, Ordering::SeqCst);
                let exit_code = status.and_then(|s| s.code());
                let _ = exit_tx.send(exit_code);
            });

            Ok((
                PtyHandle {
                    master_fd: master,
                    pid,
                    terminated,
                },
                exit_rx,
            ))
        }

        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY streams require a POSIX environment",
            ))
        }
    }

    /// Writes raw bytes into the terminal stdin.
    pub fn write_all(&self, data: &[u8]) -> io::Result<usize> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "terminal session has exited",
            ));
        }

        #[cfg(unix)]
        {
            let mut total = 0;
            while total < data.len() {
                let slice = &data[total..];
                let n = unsafe {
                    libc::write(
                        self.master_fd,
                        slice.as_ptr() as *const libc::c_void,
                        slice.len(),
                    )
                };
                if n > 0 {
                    total += n as usize;
                } else if n < 0 {
                    return Err(io::Error::last_os_error());
                } else {
                    break;
                }
            }
            Ok(total)
        }

        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY writes require POSIX",
            ))
        }
    }

    /// Resizes the pseudo-terminal dimensions.
    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        #[cfg(unix)]
        {
            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let ret = unsafe { libc::ioctl(self.master_fd, libc::TIOCSWINSZ, &ws) };
            if ret != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        #[cfg(not(unix))]
        {
            let _ = (cols, rows);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY resize requires POSIX",
            ))
        }
    }

    /// Terminates the child process.
    pub fn terminate(&self) -> io::Result<()> {
        self.terminated.store(true, Ordering::SeqCst);
        #[cfg(unix)]
        {
            unsafe {
                libc::kill(self.pid as i32, libc::SIGTERM);
                // Also close master FD to break reader loop
                libc::close(self.master_fd);
            }
            Ok(())
        }

        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY terminate requires POSIX",
            ))
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        if !self.terminated.load(Ordering::SeqCst) {
            let _ = self.terminate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn test_pty_spawn_and_write() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, mut rx) = mpsc::channel(32);
            let temp = tempfile::tempdir().unwrap();

            let (pty, _exit_rx) = PtyHandle::spawn(
                temp.path(),
                Some("echo hello_pty"),
                80,
                24,
                tx,
            )
            .unwrap();

            assert!(pty.pid() > 0);

            let mut output = String::new();
            while let Some(chunk) = rx.recv().await {
                output.push_str(&String::from_utf8_lossy(&chunk));
                if output.contains("hello_pty") {
                    break;
                }
            }
            assert!(output.contains("hello_pty"));
        });
    }

    #[test]
    #[cfg(unix)]
    fn test_pty_resize() {
        let (tx, _rx) = mpsc::channel(32);
        let temp = tempfile::tempdir().unwrap();
        let (pty, _) = PtyHandle::spawn(
            temp.path(),
            None,
            80,
            24,
            tx,
        )
        .unwrap();

        assert!(pty.resize(120, 40).is_ok());
    }
}
