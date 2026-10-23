//! Java runtime discovery.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::config::settings::Settings;
use crate::utils::paths;

/// Find a Java binary: explicit setting, launcher-managed runtimes, then `PATH`.
pub fn detect(settings: &Settings) -> Option<PathBuf> {
    if let Some(path) = settings.java_path() {
        if path.is_file() {
            return Some(path);
        }
        tracing::warn!("configured Java binary not found: {}", path.display());
    }

    if let Some(managed) = find_managed() {
        return Some(managed);
    }

    which("java")
}

/// Look for `~/.local/share/mc-launcher/java/<runtime>/bin/java`.
fn find_managed() -> Option<PathBuf> {
    let entries = std::fs::read_dir(paths::runtimes_dir()).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let candidate = entry.path().join("bin").join(java_binary_name());
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn java_binary_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

/// Minimal `which(1)`: scan `PATH` for an executable file.
pub fn which(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}

/// Run `java -version` and return the first line, e.g. `openjdk version "21.0.2"`.
pub fn probe_version(java: &Path) -> Option<String> {
    let output = std::process::Command::new(java)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stderr);
    text.lines().next().map(|line| line.trim().to_string())
}

/// The major version required by a Minecraft release, roughly.
///
/// `mc-launcher-core` will pick a runtime itself; this is only used for the UI hint.
pub fn recommended_major(mc_version: &str) -> u32 {
    let minor: u32 = mc_version
        .split('.')
        .nth(1)
        .and_then(|part| part.parse().ok())
        .unwrap_or(0);

    match minor {
        0..=16 => 8,
        17 => 16,
        18..=20 => 17,
        _ => 21,
    }
}
