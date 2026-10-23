//! Instance model and on-disk layout (Prism-style: one folder per instance).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl LoaderKind {
    pub const ALL: [LoaderKind; 5] = [
        LoaderKind::Vanilla,
        LoaderKind::Fabric,
        LoaderKind::Quilt,
        LoaderKind::Forge,
        LoaderKind::NeoForge,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LoaderKind::Vanilla => "Vanilla",
            LoaderKind::Fabric => "Fabric",
            LoaderKind::Quilt => "Quilt",
            LoaderKind::Forge => "Forge",
            LoaderKind::NeoForge => "NeoForge",
        }
    }

    pub fn is_modded(self) -> bool {
        self != LoaderKind::Vanilla
    }
}

impl Default for LoaderKind {
    fn default() -> Self {
        LoaderKind::Vanilla
    }
}

fn default_memory() -> u32 {
    4096
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// Folder name, slugified from `name`. Immutable after creation.
    pub id: String,
    pub name: String,
    pub mc_version: String,
    #[serde(default)]
    pub loader: LoaderKind,
    /// Pinned loader build. `None` means latest stable.
    #[serde(default)]
    pub loader_version: Option<String>,
    /// Version id resolved by mc-launcher-core on the last install.
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    #[serde(default)]
    pub jvm_args: String,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub created: i64,
    #[serde(default)]
    pub last_played: Option<i64>,
}

impl Instance {
    pub fn new(name: &str, mc_version: &str, loader: LoaderKind) -> Self {
        let now = now_unix();
        let base_id = slugify(name);
        let id = unique_id(&base_id);

        Self {
            id,
            name: name.trim().to_string(),
            mc_version: mc_version.trim().to_string(),
            loader,
            loader_version: None,
            version_id: None,
            memory_mb: default_memory(),
            jvm_args: String::new(),
            java_path: None,
            created: now,
            last_played: None,
        }
    }

    pub fn dir(&self) -> PathBuf {
        paths::instances_dir().join(&self.id)
    }

    /// The `.minecraft` this instance owns.
    pub fn minecraft_dir(&self) -> PathBuf {
        self.dir().join(".minecraft")
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.minecraft_dir().join("mods")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.dir().join("instance.json")
    }

    pub fn subtitle(&self) -> String {
        if self.loader.is_modded() {
            format!("{} · {}", self.mc_version, self.loader.label())
        } else {
            self.mc_version.clone()
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.minecraft_dir())?;
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(self.manifest_path(), raw)?;
        Ok(())
    }

    pub fn mark_played(&mut self) {
        self.last_played = Some(now_unix());
        if let Err(err) = self.save() {
            tracing::warn!("failed to save instance {}: {err:#}", self.id);
        }
    }

    pub fn load(id: &str) -> anyhow::Result<Self> {
        let path = paths::instances_dir().join(id).join("instance.json");
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// All instances on disk, most recently played first. Unreadable entries are skipped.
    pub fn list() -> Vec<Self> {
        let Ok(entries) = std::fs::read_dir(paths::instances_dir()) else {
            return Vec::new();
        };

        let mut instances: Vec<Self> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                match Self::load(&name) {
                    Ok(instance) => Some(instance),
                    Err(err) => {
                        tracing::warn!("skipping instance {name}: {err:#}");
                        None
                    }
                }
            })
            .collect();

        instances.sort_by(|a, b| {
            let ka = (a.last_played.unwrap_or(a.created), &a.name);
            let kb = (b.last_played.unwrap_or(b.created), &b.name);
            kb.cmp(&ka)
        });

        instances
    }

    /// Move the instance folder to the trash-less equivalent: rename, then delete.
    pub fn delete(&self) -> anyhow::Result<()> {
        std::fs::remove_dir_all(self.dir())?;
        Ok(())
    }
}

pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_dash = false;

    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "instance".to_string()
    } else {
        slug
    }
}

fn unique_id(base: &str) -> String {
    let mut candidate = base.to_string();
    let mut counter = 2;
    while paths::instances_dir().join(&candidate).exists() {
        candidate = format!("{base}-{counter}");
        counter += 1;
    }
    candidate
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
