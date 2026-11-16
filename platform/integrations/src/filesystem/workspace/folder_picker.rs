//! Native OS folder picker dialog adapter.

/// Prompts the user to pick a local directory using OS dialog primitives (`osascript` on macOS).
pub async fn pick_folder(purpose: Option<&str>) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let script = if purpose == Some("model") {
            "POSIX path of (choose folder with prompt \"Select Folder Containing GGUF Models\")"
        } else {
            "POSIX path of (choose folder with prompt \"Select Project or Repository Folder\")"
        };
        let out = tokio::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .await
            .ok()?;

        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            None
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = purpose;
        None
    }
}

