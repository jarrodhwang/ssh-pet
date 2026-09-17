use crate::{model::Config, AppError, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};
const MAX_FILE_BYTES: u64 = 512 * 1024;

/// No final-file symlinks or special files. Reads have a hard allocation limit.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(AppError::storage(error)),
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err(AppError::storage(
            "Expected a regular data file smaller than 512 KiB.",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(AppError::storage)?;
    let metadata = file.metadata().map_err(AppError::storage)?;
    if !metadata.is_file() {
        return Err(AppError::storage("Data must be a regular file."));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
            return Err(AppError::storage(
                "Data files must belong to you and have owner-only permissions (chmod 600).",
            ));
        }
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(AppError::storage)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(AppError::storage("The data file is too large."));
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(AppError::storage)
}

pub fn load(path: &Path) -> Result<Option<Config>> {
    let config: Option<Config> = read_json(path)?;
    if let Some(config) = &config {
        config.validate().map_err(AppError::from)?;
    }
    Ok(config)
}

pub fn private_directory(parent: &Path) -> Result<()> {
    if !parent.exists() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(parent).map_err(AppError::storage)?;
    }
    let metadata = fs::symlink_metadata(parent).map_err(AppError::storage)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(AppError::storage(
            "The app data folder cannot be a symlink.",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
            return Err(AppError::storage(
                "The app data folder must be owned by you and not writable by other users.",
            ));
        }
    }
    Ok(())
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::storage("Invalid data path."))?;
    private_directory(parent)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(AppError::storage(
                "Refusing to replace a symlink or special file.",
            ))
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(AppError::storage(error))
        }
        _ => {}
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(AppError::storage)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(AppError::storage("The data file is too large."));
    }
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(AppError::storage)?;
    file.write_all(&bytes).map_err(AppError::storage)?;
    file.as_file().sync_all().map_err(AppError::storage)?;
    file.persist(path).map_err(AppError::storage)?;
    Ok(())
}

pub fn save(path: &Path, config: &Config) -> Result<()> {
    config.validate().map_err(AppError::from)?;
    write_json(path, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_private_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        save(&path, &Config::default()).unwrap();
        assert!(load(&path).unwrap().unwrap().pet_visible);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn corrupt_config_remains_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        save(&path, &Config::default()).unwrap();
        fs::write(&path, "broken").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "broken");
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_redirect_settings_reads_or_writes() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original");
        let link = dir.path().join("link");
        fs::write(&original, "do not replace").unwrap();
        std::os::unix::fs::symlink(&original, &link).unwrap();
        assert!(load(&link).is_err());
        assert!(save(&link, &Config::default()).is_err());
        assert_eq!(fs::read_to_string(original).unwrap(), "do not replace");
    }
    #[test]
    fn oversized_files_are_rejected_before_deserialization() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, vec![b' '; MAX_FILE_BYTES as usize + 1]).unwrap();
        assert!(load(&path).is_err());
    }
}
