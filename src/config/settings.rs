//! Global, launcher-wide configuration.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::auth::AccountRecord;
use crate::utils::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Default heap size for new instances.
    pub memory_mb: u32,
    /// Default extra JVM flags for new instances.
    pub jvm_args: String,
    /// Explicit Java binary override. `None` means "auto-detect".
    pub java_path: Option<String>,
    /// Show snapshots / old betas in the version picker.
    pub show_snapshots: bool,
    /// Hide the launcher window once the game has started.
    pub close_on_launch: bool,
    /// Azure application (client) id used for the Microsoft device-code flow.
    pub ms_client_id: String,
    pub accounts: Vec<AccountRecord>,
    pub active_account: Option<String>,
    pub last_instance: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            memory_mb: 4096,
            jvm_args: "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions".to_string(),
            java_path: None,
            show_snapshots: false,
            close_on_launch: false,
            ms_client_id: String::new(),
            accounts: Vec::new(),
            active_account: None,
            last_instance: None,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = paths::config_file();
        match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|err| {
                tracing::warn!("invalid config at {}: {err}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = paths::config_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        // Write-then-rename so a crash mid-write cannot corrupt the config.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, raw)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn java_path(&self) -> Option<PathBuf> {
        self.java_path.as_ref().map(PathBuf::from)
    }

    pub fn account(&self, id: &str) -> Option<&AccountRecord> {
        self.accounts.iter().find(|account| account.id() == id)
    }
}
