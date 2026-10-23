//! The single seam between the launcher and `mc-launcher-core`.
//!
//! Everything in this file is blocking. Call it from a worker thread.
//!
//! ## If the crate API moved
//!
//! The calls used here are, in order:
//!
//! * `Launcher::new(PathBuf) -> Launcher`
//! * `Launcher::install(InstallRequest { minecraft_version, loader, java }) -> InstallResult`
//!   where `InstallResult::version_id: String`
//! * `Launcher::load_version(&str) -> VersionJson`
//! * `Launcher::build_launch_command_from_version(&VersionJson, LaunchOptions) -> LaunchCommand`
//! * `Account::offline(&str) -> Account`
//! * `LoaderSpec::{Fabric, Quilt, Forge, NeoForge} { version: LoaderVersion::LatestStable }`
//!
//! If any signature drifted, this file is the only place to fix.
//!
//! ## Memory and JVM arguments
//!
//! `LaunchOptions` does not (yet) expose heap settings directly, but `LaunchCommand::args` is
//! `java <JVM flags…> -cp … <main class> <game args…>`. Prepending our flags to `args` therefore
//! puts them exactly where the JVM expects them.

use std::path::{Path, PathBuf};

use mc_launcher_core::prelude::*;

use crate::config::instance::{Instance, LoaderKind};
use crate::config::settings::Settings;
use crate::core::auth::AccountRecord;

/// Map our loader enum onto the crate's install spec.
fn loader_spec(kind: LoaderKind) -> Option<LoaderSpec> {
    match kind {
        LoaderKind::Vanilla => None,
        LoaderKind::Fabric => Some(LoaderSpec::Fabric {
            version: LoaderVersion::LatestStable,
        }),
        LoaderKind::Quilt => Some(LoaderSpec::Quilt {
            version: LoaderVersion::LatestStable,
        }),
        LoaderKind::Forge => Some(LoaderSpec::Forge {
            version: LoaderVersion::LatestStable,
        }),
        LoaderKind::NeoForge => Some(LoaderSpec::NeoForge {
            version: LoaderVersion::LatestStable,
        }),
    }
}

/// Adapter from our stored account to the crate's launch identity.
///
/// TODO(api): the Microsoft arm currently falls back to an offline identity. Replace it with the
/// crate's Microsoft account constructor so servers accept the session. Offline accounts and
/// singleplayer work as-is.
fn to_core_account(record: &AccountRecord) -> Account {
    match record {
        AccountRecord::Offline { username, .. } => Account::offline(username),
        AccountRecord::Microsoft { username, .. } => Account::offline(username),
    }
}

fn java_policy(_settings: &Settings) -> JavaInstallPolicy {
    // mc-launcher-core doesn't support custom Java paths via JavaInstallPolicy
    // Custom Java is handled via LaunchOptions::java_executable instead
    JavaInstallPolicy::Auto
}

/// Result of an install pass.
pub struct Installed {
    pub version_id: String,
}

/// Resolve + download everything the instance needs. Idempotent.
pub fn ensure_installed(
    instance: &Instance,
    settings: &Settings,
    mut report: impl FnMut(&str),
) -> anyhow::Result<Installed> {
    report(&format!(
        "Resolving {} {}…",
        instance.mc_version,
        instance.loader.label()
    ));

    let launcher = Launcher::new(instance.minecraft_dir());

    let install = launcher
        .install(InstallRequest {
            minecraft_version: instance.mc_version.clone(),
            loader: loader_spec(instance.loader),
            java: java_policy(settings),
        })
        .map_err(|err| anyhow::anyhow!("install failed: {err}"))?;

    report(&format!("Version ready: {}", install.version_id));

    Ok(Installed {
        version_id: install.version_id,
    })
}

/// Build the Java command line and spawn the game.
pub fn launch(
    instance: &Instance,
    account: &AccountRecord,
    _settings: &Settings,
    version_id: &str,
    java: Option<&Path>,
    mut report: impl FnMut(&str),
) -> anyhow::Result<std::process::Child> {
    let launcher = Launcher::new(instance.minecraft_dir());

    let version = launcher
        .load_version(version_id)
        .map_err(|err| anyhow::anyhow!("could not load version {version_id}: {err}"))?;

    let mut options = LaunchOptions::default();
    options.account = to_core_account(account);
    if let Some(path) = java {
        options.java_executable = Some(path.to_path_buf());
    }

    let command = launcher
        .build_launch_command_from_version(&version, options)
        .map_err(|err| anyhow::anyhow!("could not build launch command: {err}"))?;

    let jvm_flags = build_jvm_flags(instance);
    report(&format!("JVM flags: {}", jvm_flags.join(" ")));

    let mut args: Vec<String> = jvm_flags;
    args.extend(command.args.iter().cloned());

    report(&format!("Launching {}", instance.name));

    let child = std::process::Command::new(&command.executable)
        .args(&args)
        .current_dir(&command.working_dir)
        .spawn()
        .map_err(|err| anyhow::anyhow!("could not start Java: {err}"))?;

    Ok(child)
}

/// `-Xmx` / `-Xms` plus whatever the instance configured.
fn build_jvm_flags(instance: &Instance) -> Vec<String> {
    let heap = instance.memory_mb.max(512);
    let initial = (heap / 2).max(512).min(heap);

    let mut flags = vec![format!("-Xms{initial}M"), format!("-Xmx{heap}M")];
    flags.extend(split_args(&instance.jvm_args));
    flags
}

/// Split a JVM argument string, honouring double quotes.
pub fn split_args(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut has_content = false;

    for ch in raw.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                has_content = true;
            }
            c if c.is_whitespace() && !quoted => {
                if has_content {
                    out.push(std::mem::take(&mut current));
                    has_content = false;
                }
            }
            c => {
                current.push(c);
                has_content = true;
            }
        }
    }

    if has_content {
        out.push(current);
    }

    out
}

/// Path of the Java binary that will be used, best-effort.
pub fn resolve_java(instance: &Instance, settings: &Settings) -> Option<PathBuf> {
    if let Some(path) = instance.java_path.as_ref().map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    crate::core::java::detect(settings)
}
