use droplet_core::{security::LaunchSpec, AppError, ErrorCode, Result};

#[cfg(target_os = "macos")]
pub async fn launch(spec: &LaunchSpec) -> Result<()> {
    use std::{process::Stdio, time::Duration};
    // Fixed AppleScript source, fully quoted SSH words passed only as argv.
    let script = r#"on run argv
        with timeout of 30 seconds
            tell application "Terminal"
                activate
                do script (item 1 of argv)
            end tell
        end timeout
        return
    end run"#;
    let mut process = tokio::process::Command::new("/usr/bin/osascript");
    process
        .args(["-e", script, &spec.shell_command()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(35), process.output()).await
        .map_err(|_| AppError::new(ErrorCode::Process, "Terminal took too long to respond. Check whether a Terminal window opened before trying again."))?
        .map_err(|e| AppError::new(ErrorCode::Process, format!("Could not open Terminal: {e}")))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let message = if detail.contains("-1743") {
            "Allow Droplet to control Terminal in System Settings → Privacy & Security → Automation, then try again."
        } else {
            "Terminal could not open the connection. Check Terminal and macOS Automation permissions before retrying."
        };
        return Err(AppError::new(ErrorCode::Process, message));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub async fn launch(_spec: &LaunchSpec) -> Result<()> {
    Err(AppError::new(ErrorCode::Unsupported, "Terminal launching is available on macOS in this version. Windows and Linux adapters are planned."))
}
