//! Authored setup before product stores are assembled.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::config::{xdg, ConfigLoader, PhysicalBinding};
use crate::error::ApiError;

/// Configuration selection and the preparation lock held through native genesis.
pub struct PreparedConfiguration {
    /// Selected configuration, authored only for a fresh installation.
    pub path: PathBuf,
    _lock: File,
}

/// Preserve configured products, or author a fresh bundled Startup selection.
pub fn prepare_configuration(
    explicit: Option<&Path>,
    package: Option<&Path>,
) -> Result<PreparedConfiguration, ApiError> {
    let path = match explicit {
        Some(path) => std::path::absolute(path).map_err(error)?,
        None => xdg::config_home()?.join("meld/config.toml"),
    };
    let parent = path
        .parent()
        .ok_or_else(|| error("configuration needs a parent directory"))?;
    if package.is_some() && !path.is_file() {
        return Err(error("--package requires an existing native product configuration; use plain meld init for bundled Startup"));
    }
    if !path.exists() && explicit.is_none() && parent.exists() {
        let has_existing = fs::read_dir(parent)
            .map_err(error)?
            .any(|entry| entry.map_or(true, |entry| entry.file_name() != ".init.lock"));
        if has_existing {
            return Err(error("existing Meld configuration is preserved; use meld --config /absolute/new/config.toml init for a separate Startup installation"));
        }
    }
    if path.exists() {
        validate_native_configuration(&path)?;
    }
    fs::create_dir_all(parent).map_err(error)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(parent.join(".init.lock"))
        .map_err(error)?;
    FileExt::try_lock_exclusive(&lock)
        .map_err(|_| error("another initialization is in progress; retry after it completes"))?;
    if !path.exists() {
        let data = xdg::data_home().ok_or_else(|| error("cannot resolve XDG data home"))?;
        let identity = blake3::hash(path.as_os_str().as_encoded_bytes()).to_hex();
        let product = data
            .join("meld/products")
            .join(format!("startup-{}", &identity[..12]));
        if product.exists() {
            return Err(error(format!(
                "unconfigured product state at {} is preserved; select a different --config path",
                product.display()
            )));
        }
        let mut preset: toml::Value =
            toml::from_str(include_str!("../../theory/startup/bootstrap.toml")).map_err(error)?;
        preset["system"]["storage"]["product_root"] =
            toml::Value::String(product.to_string_lossy().into_owned());
        let body = toml::to_string_pretty(&preset).map_err(error)?;
        let mut pending = tempfile::Builder::new()
            .suffix(".toml")
            .tempfile_in(parent)
            .map_err(error)?;
        pending.write_all(body.as_bytes()).map_err(error)?;
        pending.as_file().sync_all().map_err(error)?;
        validate_native_configuration(pending.path())?;
        pending.persist_noclobber(&path).map_err(error)?;
    }
    validate_native_configuration(&path)?;
    Ok(PreparedConfiguration { path, _lock: lock })
}

fn validate_native_configuration(path: &Path) -> Result<(), ApiError> {
    let config = ConfigLoader::load_from_file(path)?;
    if PhysicalBinding::resolve_all(&config)?.is_empty() {
        return Err(error(format!("{} has no native stewardship declaration and is preserved; use meld --config /absolute/new/config.toml init for fresh Startup", path.display())));
    }
    Ok(())
}

fn error(message: impl std::fmt::Display) -> ApiError {
    ApiError::ConfigError(format!("initialization: {message}"))
}
