//! Player avatar cache (mc-heads.net).

use std::path::PathBuf;
use std::time::Duration;

use crate::utils::paths;

/// Return a local PNG path for the avatar, downloading it on first use.
///
/// `key` is a UUID (Microsoft accounts) or a player name (offline accounts).
pub fn avatar_path(key: &str) -> Option<PathBuf> {
    let safe = sanitize(key);
    if safe.is_empty() {
        return None;
    }

    let dir = paths::cache_dir().join("skins");
    std::fs::create_dir_all(&dir).ok()?;

    let file = dir.join(format!("{safe}.png"));
    if file.is_file() {
        return Some(file);
    }

    let url = format!("https://mc-heads.net/avatar/{safe}/128");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("mc-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let bytes = client.get(url).send().ok()?.bytes().ok()?;
    std::fs::write(&file, &bytes).ok()?;
    Some(file)
}

fn sanitize(key: &str) -> String {
    key.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect()
}
