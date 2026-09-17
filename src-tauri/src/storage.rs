use crate::model::Config;
use std::{fs, io::Write, path::Path};

pub fn load(path: &Path) -> Result<Option<Config>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Could not read settings: {error}")),
    };
    let config: Config = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Could not read settings: {error}"))?;
    config.validate()?;
    Ok(Some(config))
}

/// Write beside the destination and atomically replace it, so a partial write cannot lose connections.
pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    let parent = path.parent().ok_or("Invalid settings path.")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create the settings folder: {error}"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    file.persist(path)
        .map_err(|error| format!("Could not save settings: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_replace() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        assert!(load(&path).unwrap().is_none());
        let mut config = Config::default();
        save(&path, &config).unwrap();
        config.pet_visible = false;
        save(&path, &config).unwrap();
        assert!(!load(&path).unwrap().unwrap().pet_visible);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn corrupt_config_is_reported_and_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(&path, "broken config").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "broken config");
    }
}
