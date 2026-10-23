//! XDG base directories and application layout.
//!
//! Deliberately dependency-free: reading `XDG_*` and `HOME` directly is less code than
//! pulling in a crate and behaves identically for our purposes.

use std::path::PathBuf;

pub const APP_ID: &str = "com.betterdeepseek.Launcher";

/// Folder name under the XDG base dirs.
const APP_DIR: &str = "mc-launcher";

fn base(env_var: &str, fallback: &str) -> PathBuf {
    if let Some(value) = std::env::var_os(env_var) {
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    home.join(fallback)
}

/// `~/.local/share/mc-launcher`
pub fn data_dir() -> PathBuf {
    base("XDG_DATA_HOME", ".local/share").join(APP_DIR)
}

/// `~/.config/mc-launcher`
pub fn config_dir() -> PathBuf {
    base("XDG_CONFIG_HOME", ".config").join(APP_DIR)
}

/// `~/.cache/mc-launcher`
pub fn cache_dir() -> PathBuf {
    base("XDG_CACHE_HOME", ".cache").join(APP_DIR)
}

pub fn instances_dir() -> PathBuf {
    data_dir().join("instances")
}

/// Java runtimes downloaded/managed by the launcher.
pub fn runtimes_dir() -> PathBuf {
    data_dir().join("java")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

pub fn create_all(dirs: &[PathBuf]) -> std::io::Result<()> {
    for dir in dirs {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub fn ensure_dirs() -> std::io::Result<()> {
    create_all(&[
        data_dir(),
        config_dir(),
        cache_dir(),
        instances_dir(),
        runtimes_dir(),
        cache_dir().join("skins"),
    ])
}
