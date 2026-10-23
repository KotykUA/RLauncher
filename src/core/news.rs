//! Mojang patch notes and a rough service reachability check.

use std::time::Duration;

const PATCH_NOTES_URL: &str = "https://launchercontent.mojang.com/v2/javaPatchNotes.json";
const SERVICES_URL: &str = "https://api.minecraftservices.com/";

fn client() -> anyhow::Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("mc-launcher/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

#[derive(Debug, Clone)]
pub struct NewsEntry {
    pub title: String,
    pub version: String,
    pub body: String,
    pub url: Option<String>,
}

/// Java Edition patch notes. Blocking.
pub fn fetch_news(limit: usize) -> anyhow::Result<Vec<NewsEntry>> {
    let json = client()?
        .get(PATCH_NOTES_URL)
        .send()?
        .error_for_status()?
        .json::<serde_json::Value>()?;

    let entries = json
        .get("entries")
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("unexpected patch notes payload"))?;

    let news = entries
        .iter()
        .take(limit)
        .map(|entry| {
            let text = |key: &str| {
                entry
                    .get(key)
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string()
            };

            let body = entry
                .get("body")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .replace('\r', "")
                .lines()
                .take(12)
                .collect::<Vec<_>>()
                .join("\n");

            let path = entry.get("path").and_then(|value| value.as_str());

            NewsEntry {
                title: text("title"),
                version: text("version"),
                body,
                url: path.map(|p| format!("https://www.minecraft.net/en-us/article/{p}")),
            }
        })
        .collect();

    Ok(news)
}

/// Best-effort reachability of the Minecraft services API.
pub fn service_status() -> String {
    match client().and_then(|client| {
        client
            .get(SERVICES_URL)
            .send()
            .map_err(anyhow::Error::from)
    }) {
        Ok(response) if response.status().is_server_error() => {
            format!("Degraded ({})", response.status())
        }
        Ok(_) => "Operational".to_string(),
        Err(err) => format!("Unreachable ({err})"),
    }
}
