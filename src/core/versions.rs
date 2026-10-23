//! Mojang's version manifest.

use std::time::Duration;

use serde::Deserialize;

const MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseKind {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

impl ReleaseKind {
    pub fn label(self) -> &'static str {
        match self {
            ReleaseKind::Release => "release",
            ReleaseKind::Snapshot => "snapshot",
            ReleaseKind::OldBeta => "old_beta",
            ReleaseKind::OldAlpha => "old_alpha",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub release_time: String,
}

impl VersionEntry {
    pub fn is_snapshot(&self) -> bool {
        self.kind != "release"
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    versions: Vec<VersionEntry>,
}

/// Fetch the full version list. Blocking — call from a worker thread.
pub fn fetch_all() -> anyhow::Result<Vec<VersionEntry>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("mc-launcher/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let manifest = client
        .get(MANIFEST_URL)
        .send()?
        .error_for_status()?
        .json::<Manifest>()?;

    Ok(manifest.versions)
}

/// Releases first, then optionally snapshots. Newest first, as Mojang provides them.
pub fn filtered(include_snapshots: bool) -> anyhow::Result<Vec<VersionEntry>> {
    let all = fetch_all()?;
    Ok(all
        .into_iter()
        .filter(|entry| include_snapshots || !entry.is_snapshot())
        .collect())
}
