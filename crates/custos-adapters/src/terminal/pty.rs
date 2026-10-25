//! Native POSIX PTY Process Management
//!
//! Spawns real pseudo-terminal processes on Unix (macOS / Linux) with:
//! - Master / slave pair creation via `libc::openpty`
//! - Controlling terminal setup and standard IO redirection
//! - Dynamic window resizing via `TIOCSWINSZ`
//! - Raw non-blocking / streaming I/O with process lifecycle observation

use async_trait::async_trait;
use custos_core::contracts::terminal::{
    TerminalDriverPort, TerminalOutputConsumer, TerminalProcessHandle,
};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::debug;

#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// Native PTY handle managing a spawned child process and its master PTY descriptor.
pub struct PtyHandle {
    #[cfg(unix)]
    master_fd: OwnedFd,
    #[cfg(windows)]
    stdin: Arc<std::sync::Mutex<std::process::ChildStdin>>,
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
        consumer: Arc<dyn TerminalOutputConsumer>,
    ) -> io::Result<Self> {
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
            // Both ends need independent ownership; process exit does not close the reader.
            let master_owned = unsafe { OwnedFd::from_raw_fd(master) };
            let reader_fd = master_owned.try_clone();
            let reader_fd = match reader_fd {
                Ok(fd) => fd,
                Err(error) => {
                    unsafe {
                        libc::close(slave);
                    }
                    return Err(error);
                }
            };
            unsafe {
                libc::fcntl(master, libc::F_SETFD, libc::FD_CLOEXEC);
            }

            // Determine shell binary with macOS (/bin/zsh) and Linux (/bin/bash) fallback
            let default_shell = std::env::var("SHELL").unwrap_or_else(|_| {
                #[cfg(target_os = "macos")]
                {
                    "/bin/zsh".to_string()
                }
                #[cfg(not(target_os = "macos"))]
                {
                    "/bin/bash".to_string()
                }
            });

            let (prog, args) = match command_override {
                Some(cmd) if !cmd.trim().is_empty() => (
                    default_shell.clone(),
                    vec!["-c".to_string(), cmd.to_string()],
                ),
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
                    }
                    return Err(e);
                }
            };

            let pid = child.id();
            let terminated = Arc::new(AtomicBool::new(false));
            let term_flag = terminated.clone();

            let out_consumer = consumer.clone();
            // Background reader thread reading raw PTY stream
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    let n = unsafe {
                        libc::read(
                            reader_fd.as_raw_fd(),
                            buf.as_mut_ptr() as *mut libc::c_void,
                            buf.len(),
                        )
                    };

                    if n > 0 {
                        out_consumer.on_output(&buf[..n as usize]);
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

            let exit_consumer = consumer;
            // Background waiter thread observing child process exit
            std::thread::spawn(move || {
                let status = child.wait().ok();
                term_flag.store(true, Ordering::SeqCst);
                let exit_code = status.and_then(|s| s.code());
                exit_consumer.on_exit(exit_code);
            });

            Ok(PtyHandle {
                master_fd: master_owned,
                pid,
                terminated,
            })
        }

        #[cfg(windows)]
        {
            use std::io::Read;
            use std::process::{Command, Stdio};

            let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string());
            let (prog, args) = match command_override {
                Some(cmd) if !cmd.trim().is_empty() => {
                    if comspec.to_lowercase().ends_with("cmd.exe") {
                        (comspec, vec!["/c".to_string(), cmd.to_string()])
                    } else {
                        (
                            comspec,
                            vec![
                                "-NoProfile".to_string(),
                                "-Command".to_string(),
                                cmd.to_string(),
                            ],
                        )
                    }
                }
                _ => {
                    if comspec.to_lowercase().ends_with("cmd.exe") {
                        (comspec, vec![])
                    } else {
                        (comspec, vec!["-NoLogo".to_string()])
                    }
                }
            };

            let mut cmd = Command::new(&prog);
            cmd.args(&args);
            cmd.current_dir(working_dir);
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut child = cmd.spawn()?;
            let pid = child.id();
            let stdin = child.stdin.take().ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Failed to capture child stdin")
            })?;
            let mut stdout = child.stdout.take().ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Failed to capture child stdout")
            })?;
            let mut stderr = child.stderr.take().ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Failed to capture child stderr")
            })?;

            let terminated = Arc::new(AtomicBool::new(false));
            let term_flag = terminated.clone();

            let out_c1 = consumer.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                while let Ok(n) = stdout.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    out_c1.on_output(&buf[..n]);
                }
            });

            let out_c2 = consumer.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                while let Ok(n) = stderr.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    out_c2.on_output(&buf[..n]);
                }
            });

            let exit_c = consumer;
            std::thread::spawn(move || {
                let status = child.wait().ok();
                term_flag.store(true, Ordering::SeqCst);
                let exit_code = status.and_then(|s| s.code());
                exit_c.on_exit(exit_code);
            });

            Ok(PtyHandle {
                stdin: Arc::new(std::sync::Mutex::new(stdin)),
                pid,
                terminated,
            })
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = (working_dir, command_override, cols, rows, consumer);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY streams require a Unix or Windows environment",
            ))
        }
    }
}

impl TerminalProcessHandle for PtyHandle {
    fn write_all(&self, data: &[u8]) -> io::Result<usize> {
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
                        self.master_fd.as_raw_fd(),
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

        #[cfg(windows)]
        {
            use std::io::Write;
            let mut lock = self.stdin.lock().map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "failed to lock child stdin")
            })?;
            lock.write_all(data)?;
            lock.flush()?;
            Ok(data.len())
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = data;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY writes require Unix or Windows",
            ))
        }
    }

    fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        #[cfg(unix)]
        {
            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let ret = unsafe { libc::ioctl(self.master_fd.as_raw_fd(), libc::TIOCSWINSZ, &ws) };
            if ret != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            let _ = (cols, rows);
            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = (cols, rows);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY resize requires Unix",
            ))
        }
    }

    fn terminate(&self) -> io::Result<()> {
        if self.terminated.load(Ordering::SeqCst) {
            return Ok(());
        }
        #[cfg(unix)]
        {
            let result = unsafe { libc::kill(-(self.pid as i32), libc::SIGTERM) };
            if result != 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            let output = std::process::Command::new("taskkill")
                .args(["/PID", &self.pid.to_string(), "/T", "/F"])
                .output()?;
            if !output.status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY terminate requires Unix or Windows",
            ))
        }
    }

    fn force_terminate(&self) -> io::Result<()> {
        if self.has_exited() {
            return Ok(());
        }
        #[cfg(unix)]
        {
            if unsafe { libc::kill(-(self.pid as i32), libc::SIGKILL) } != 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            self.terminate()
        }
    }

    fn pid(&self) -> u32 {
        self.pid
    }

    fn has_exited(&self) -> bool {
        self.terminated.load(Ordering::SeqCst)
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        if !self.terminated.load(Ordering::SeqCst) {
            let _ = self.terminate();
        }
    }
}

/// Native pseudo-terminal driver implementing `TerminalDriverPort`.
#[derive(Clone, Default)]
pub struct NativePtyDriver;

#[async_trait]
impl TerminalDriverPort for NativePtyDriver {
    async fn spawn_pty(
        &self,
        working_dir: &Path,
        command_override: Option<&str>,
        cols: u16,
        rows: u16,
        consumer: Arc<dyn TerminalOutputConsumer>,
    ) -> io::Result<Box<dyn TerminalProcessHandle>> {
        let handle = PtyHandle::spawn(working_dir, command_override, cols, rows, consumer)?;
        Ok(Box::new(handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockConsumer {
        output: Arc<std::sync::Mutex<Vec<u8>>>,
        exited: Arc<AtomicBool>,
    }

    impl TerminalOutputConsumer for MockConsumer {
        fn on_output(&self, chunk: &[u8]) {
            self.output.lock().unwrap().extend_from_slice(chunk);
        }

        fn on_exit(&self, _exit_code: Option<i32>) {
            self.exited.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_pty_spawn_and_write() {
        let temp = tempfile::tempdir().unwrap();
        let consumer = Arc::new(MockConsumer {
            output: Arc::new(std::sync::Mutex::new(Vec::new())),
            exited: Arc::new(AtomicBool::new(false)),
        });

        let pty = PtyHandle::spawn(temp.path(), Some("echo hello_pty"), 80, 24, consumer.clone()).unwrap();

        assert!(pty.pid() > 0);
        assert_eq!(pty.resize(100, 30).is_ok(), true);

        // Allow child process to finish echo and exit
        for _ in 0..50 {
            if consumer.exited.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let out = consumer.output.lock().unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("hello_pty"), "PTY output did not contain expected text; got: {s}");
    }
}
