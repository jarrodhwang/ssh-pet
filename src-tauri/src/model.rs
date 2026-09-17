use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    pub port: u16,
    pub identity_file: String,
}

impl Connection {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 100
            || self.name.chars().any(char::is_control)
        {
            return Err("Give this connection a name of 1–100 characters.".into());
        }
        if self.host.is_empty()
            || self.host.len() > 253
            || self.host.starts_with('-')
            || !self
                .host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-:[]_".contains(&c))
        {
            return Err(
                "Enter a hostname, SSH alias, or IP address, without spaces or a protocol.".into(),
            );
        }
        if self.username.len() > 64
            || self.username.starts_with('-')
            || !self
                .username
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        {
            return Err(
                "Use a username containing letters, numbers, dots, underscores, or hyphens.".into(),
            );
        }
        if self.port == 0 {
            return Err("The port must be between 1 and 65535.".into());
        }
        if self.identity_file.len() > 4096 || self.identity_file.chars().any(char::is_control) {
            return Err("The key path contains unsupported characters.".into());
        }
        if !self.identity_file.is_empty()
            && !self.identity_file.starts_with("~/")
            && !Path::new(&self.identity_file).is_absolute()
        {
            return Err("Use an absolute key path or a path starting with ~/.".into());
        }
        Ok(())
    }

    pub fn destination(&self) -> String {
        if self.username.is_empty() {
            self.host.clone()
        } else {
            format!("{}@{}", self.username, self.host)
        }
    }

    pub fn identity_path(&self, home: &Path) -> Option<PathBuf> {
        if self.identity_file.is_empty() {
            None
        } else if let Some(relative) = self.identity_file.strip_prefix("~/") {
            Some(home.join(relative))
        } else {
            Some(PathBuf::from(&self.identity_file))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PetPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub version: u32,
    pub connections: Vec<Connection>,
    pub favorite_id: Option<String>,
    pub pet_visible: bool,
    pub reduce_motion: bool,
    pub pet_position: Option<PetPosition>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            connections: vec![],
            favorite_id: None,
            pet_visible: true,
            reduce_motion: false,
            pet_position: None,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This settings file was created by a different version of Droplet.".into());
        }
        if self.connections.len() > 100 {
            return Err("Droplet supports up to 100 connections.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for connection in &self.connections {
            connection.validate()?;
            if connection.id.is_empty() || !ids.insert(&connection.id) {
                return Err("Connection IDs must be unique.".into());
            }
        }
        if self
            .favorite_id
            .as_ref()
            .is_some_and(|id| !ids.contains(id))
        {
            return Err("The favorite connection could not be found.".into());
        }
        Ok(())
    }
}

/// Import a single plain SSH invocation as data. Never run the file or accept extra commands.
pub fn parse_launcher(content: &str, name: &str) -> Result<Connection, String> {
    let lines: Vec<_> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if lines.len() != 1 {
        return Err(
            "Import needs one plain ssh command. Comments and blank lines are okay.".into(),
        );
    }
    let words =
        shell_words::split(lines[0]).map_err(|_| "The SSH command has unmatched quotes.")?;
    if !matches!(
        words.first().map(String::as_str),
        Some("ssh" | "/usr/bin/ssh")
    ) {
        return Err("The launcher must start with ssh.".into());
    }
    let mut connection = Connection {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.into(),
        host: String::new(),
        username: String::new(),
        port: 22,
        identity_file: String::new(),
    };
    let mut index = 1;
    let mut destination = None;
    while index < words.len() {
        if destination.is_some() {
            return Err("The destination must be the last part of the SSH command. Remote commands are not imported.".into());
        }
        match words[index].as_str() {
            "-i" | "-p" => {
                let option = &words[index];
                index += 1;
                let value = words.get(index).ok_or("An SSH option is missing its value.")?;
                if option == "-i" { connection.identity_file = value.clone(); }
                else { connection.port = value.parse().map_err(|_| "Invalid SSH port.")?; }
            }
            "--" if destination.is_none() && index + 2 == words.len() => {
                index += 1;
                destination = Some(words[index].clone());
            }
            value if !value.starts_with('-') && destination.is_none() => destination = Some(value.to_owned()),
            _ => return Err("Only a destination, -i key path, and -p port can be imported. Add other SSH options in ~/.ssh/config.".into()),
        }
        index += 1;
    }
    let destination = destination.ok_or("The SSH command needs a destination.")?;
    if let Some((username, host)) = destination.split_once('@') {
        connection.username = username.into();
        connection.host = host.into();
    } else {
        connection.host = destination;
    }
    connection.validate()?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_plain_launcher_without_executing_it() {
        let result = parse_launcher(
            "#!/bin/bash\nssh -i ~/.ssh/id_ed25519 dev@100.64.0.2\n",
            "Studio",
        )
        .unwrap();
        assert_eq!(result.username, "dev");
        assert_eq!(result.host, "100.64.0.2");
        assert_eq!(
            result.identity_path(Path::new("/Users/dev")).unwrap(),
            PathBuf::from("/Users/dev/.ssh/id_ed25519")
        );
    }

    #[test]
    fn import_rejects_scripts_remote_commands_and_options() {
        for command in [
            "ssh host\nrm -rf /tmp/example",
            "ssh host; whoami",
            "ssh host whoami",
            "ssh host -i /tmp/key",
            "ssh -oProxyCommand=evil host",
            "ssh $(whoami)",
            "ssh host && echo hi",
            "ssh -p 0 host",
            "curl example.com",
        ] {
            assert!(
                parse_launcher(command, "Test").is_err(),
                "accepted {command}"
            );
        }
    }

    #[test]
    fn import_handles_quoted_keys_and_ipv6() {
        let result =
            parse_launcher("ssh -p 2222 -i '/Users/dev/My Keys/id' dev@::1", "IPv6").unwrap();
        assert_eq!(result.port, 2222);
        assert_eq!(result.identity_file, "/Users/dev/My Keys/id");
        assert_eq!(result.host, "::1");
    }

    #[test]
    fn aliases_can_use_ssh_config_username() {
        assert_eq!(
            parse_launcher("ssh studio", "Studio")
                .unwrap()
                .destination(),
            "studio"
        );
    }

    #[test]
    fn validation_rejects_option_and_control_character_injection() {
        let mut connection = parse_launcher("ssh dev@studio", "Studio").unwrap();
        connection.host = "-oProxyCommand=evil".into();
        assert!(connection.validate().is_err());
        connection.host = "studio".into();
        connection.identity_file = "/tmp/id\nwhoami".into();
        assert!(connection.validate().is_err());
    }
}
