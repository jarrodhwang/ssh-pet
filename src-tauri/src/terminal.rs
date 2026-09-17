use crate::model::Connection;
use std::path::Path;

/// A POSIX shell word, including embedded quotes. User data never becomes shell syntax.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn command(connection: &Connection, home: &Path) -> Result<String, String> {
    connection.validate()?;
    let mut words = vec!["ssh".into()];
    if let Some(path) = connection.identity_path(home) {
        words.extend(["-i".into(), shell_quote(&path.to_string_lossy())]);
    }
    words.extend(["-p".into(), connection.port.to_string()]);
    words.extend(["--".into(), shell_quote(&connection.destination())]);
    Ok(words.join(" "))
}

#[cfg(target_os = "macos")]
pub fn launch(connection: &Connection, home: &Path) -> Result<(), String> {
    use std::process::Command;
    let command = command(connection, home)?;
    if let Some(path) = connection.identity_path(home) {
        if !path.is_file() {
            return Err(format!(
                "The SSH key was not found at {}. Edit the connection to choose its location.",
                path.display()
            ));
        }
    }
    // Pass the fully quoted SSH invocation as argv, never as AppleScript source.
    // Terminal handles authentication and host verification using the user's OpenSSH settings.
    let script = r#"on run argv
        with timeout of 30 seconds
            tell application "Terminal"
                activate
                do script (item 1 of argv)
            end tell
        end timeout
    end run"#;
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", script, &format!("/usr/bin/{command}")])
        .output()
        .map_err(|error| format!("Could not open Terminal: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        if detail.contains("-1743") {
            return Err("Allow Droplet to control Terminal in System Settings → Privacy & Security → Automation, then try again.".into());
        }
        return Err(format!(
            "Terminal could not open the connection: {}",
            detail.trim()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn launch(_connection: &Connection, _home: &Path) -> Result<(), String> {
    Err("Terminal launching is available on macOS in this version. Windows and Linux adapters are planned.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse_launcher;

    #[test]
    fn quoting_round_trips_shell_metacharacters_as_literal_data() {
        for path in [
            "/tmp/a key",
            "/tmp/key'quote",
            "/tmp/$(touch injected)",
            "/tmp/`whoami`",
            "/tmp/key; echo bad",
        ] {
            let mut connection = parse_launcher("ssh dev@studio", "Test").unwrap();
            connection.identity_file = path.into();
            let words = shell_words::split(&command(&connection, Path::new("/Users/dev")).unwrap())
                .unwrap();
            assert_eq!(words, ["ssh", "-i", path, "-p", "22", "--", "dev@studio"]);
        }
    }

    #[test]
    fn quoted_command_cannot_execute_key_path_substitutions() {
        #[cfg(unix)]
        {
            let directory = tempfile::tempdir().unwrap();
            let marker = directory.path().join("injected");
            let payload = format!(
                "/tmp/key'$(touch {})`touch {}`;",
                marker.display(),
                marker.display()
            );
            let output = std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(format!("printf '%s' {}", shell_quote(&payload)))
                .output()
                .unwrap();
            assert_eq!(String::from_utf8(output.stdout).unwrap(), payload);
            assert!(!marker.exists());
        }
    }
}
