use crate::{model::Connection, AppError, ErrorCode, Result};
use std::path::{Path, PathBuf};

pub fn authorize(window: &str, required: &str) -> Result<()> {
    if window != required {
        return Err(AppError::new(
            ErrorCode::Forbidden,
            "This window is not allowed to perform that action.",
        ));
    }
    Ok(())
}

/// Metadata only. Key bytes and passphrases are never loaded into Droplet.
pub fn check_identity(connection: &Connection, home: &Path) -> Result<Option<PathBuf>> {
    let Some(path) = connection.identity_path(home) else {
        return Ok(None);
    };
    if path.to_string_lossy().contains('%') || path.to_string_lossy().contains("${") {
        return Err(AppError::new(ErrorCode::UnsafeKey, "Explicit key paths cannot contain OpenSSH expansion tokens (% or ${…). Select a literal path."));
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| {
        AppError::new(
            ErrorCode::UnsafeKey,
            "The selected SSH key cannot be found or inspected. Check its path.",
        )
    })?;
    if !metadata.file_type().is_file() {
        return Err(AppError::new(ErrorCode::UnsafeKey, "Select a regular SSH key file. Symlinks, directories, and special files are not accepted."));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
            return Err(AppError::new(ErrorCode::UnsafeKey, "The SSH key must belong to you and be private. Set its permissions to 600 or 400 before connecting."));
        }
    }
    Ok(Some(path))
}

#[derive(Clone, Debug)]
pub struct LaunchSpec {
    pub program: &'static str,
    pub args: Vec<String>,
}
impl LaunchSpec {
    pub fn shell_command(&self) -> String {
        std::iter::once(self.program.to_owned())
            .chain(self.args.iter().map(|arg| {
                if cfg!(windows) {
                    powershell_quote(arg)
                } else {
                    shell_quote(arg)
                }
            }))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn cmd_command(&self) -> String {
        std::iter::once(self.program.to_owned())
            .chain(self.args.iter().map(|arg| cmd_quote(arg)))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn cmd_quote(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '^' | '&' | '|' | '<' | '>' | '(' | ')' | '!') {
            escaped.push('^');
        }
        escaped.push(character);
    }
    format!("\"{}\"", escaped.replace('"', "\\\""))
}

pub fn launch_spec(connection: &Connection, home: &Path) -> Result<LaunchSpec> {
    connection.validate().map_err(AppError::from)?;
    check_identity(connection, home)?;
    if home.to_string_lossy().contains("${") {
        return Err(AppError::new(
            ErrorCode::UnsafeKey,
            "The home path contains an OpenSSH environment expansion token.",
        ));
    }
    let mut args = Vec::new();
    // SSH config remains trusted local code: ProxyCommand and Match exec are still available.
    for option in [
        "StrictHostKeyChecking=ask",
        "ForwardAgent=no",
        "ForwardX11=no",
        "PermitLocalCommand=no",
        "ClearAllForwardings=yes",
        "RemoteCommand=none",
        "ConnectTimeout=10",
        "ConnectionAttempts=1",
        "ServerAliveInterval=30",
        "ServerAliveCountMax=3",
    ] {
        args.extend(["-o".into(), option.into()]);
    }
    let known_hosts = home
        .join(".ssh/known_hosts")
        .to_string_lossy()
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    args.extend(["-o".into(), format!("UserKnownHostsFile=\"{known_hosts}\"")]);
    if let Some(path) = connection.identity_path(home) {
        args.extend([
            "-i".into(),
            path.to_string_lossy().into_owned(),
            "-o".into(),
            "IdentitiesOnly=yes".into(),
        ]);
    }
    args.extend([
        "-p".into(),
        connection.port.to_string(),
        "--".into(),
        connection.destination(),
    ]);
    Ok(LaunchSpec {
        program: if cfg!(windows) {
            "ssh.exe"
        } else {
            "/usr/bin/ssh"
        },
        args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse_launcher;
    #[test]
    fn only_the_designated_window_can_issue_privileged_actions() {
        assert!(authorize("main", "main").is_ok());
        for origin in ["pet", "unknown", "https://evil.invalid"] {
            assert_eq!(
                authorize(origin, "main").unwrap_err().code,
                ErrorCode::Forbidden
            );
        }
    }
    #[test]
    fn policy_is_explicit_and_cannot_be_omitted_by_the_renderer() {
        let c = parse_launcher("ssh dev@studio", "Studio").unwrap();
        let spec = launch_spec(&c, Path::new("/Users/dev")).unwrap();
        for policy in [
            "StrictHostKeyChecking=ask",
            "ForwardAgent=no",
            "PermitLocalCommand=no",
            "ClearAllForwardings=yes",
        ] {
            assert!(spec.args.iter().any(|s| s == policy));
        }
        assert_eq!(&spec.args[spec.args.len() - 2..], &["--", "dev@studio"]);
    }
    #[cfg(unix)]
    #[test]
    fn unsafe_keys_are_blocked_without_changing_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let key = dir.path().join("key");
        std::fs::write(&key, "test").unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap();
        let mut c = parse_launcher("ssh studio", "Studio").unwrap();
        c.identity_file = key.to_string_lossy().into();
        assert_eq!(
            launch_spec(&c, dir.path()).unwrap_err().code,
            ErrorCode::UnsafeKey
        );
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(launch_spec(&c, dir.path()).is_ok());
    }
    #[cfg(unix)]
    #[test]
    fn shell_substitutions_remain_literal() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("injected");
        let payload = format!(
            "/tmp/key'$(touch {})`touch {}`;",
            marker.display(),
            marker.display()
        );
        let out = std::process::Command::new("/bin/sh")
            .args(["-c", &format!("printf '%s' {}", shell_quote(&payload))])
            .output()
            .unwrap();
        assert_eq!(String::from_utf8(out.stdout).unwrap(), payload);
        assert!(!marker.exists());
    }
}
