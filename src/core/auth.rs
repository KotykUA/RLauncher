//! Account identities and the Microsoft device-code login chain.
//!
//! Offline accounts are trivial. Microsoft accounts go through:
//!
//! ```text
//! device code -> MSA access token -> Xbox Live (XBL) -> XSTS -> Minecraft services -> profile
//! ```
//!
//! Every request here is blocking and must be called from a worker thread.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MSA_SCOPE: &str = "XboxLive.signin offline_access";

const USER_AGENT: &str = concat!("mc-launcher/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AccountRecord {
    Offline {
        id: String,
        username: String,
    },
    Microsoft {
        id: String,
        username: String,
        uuid: String,
        access_token: String,
        #[serde(default)]
        refresh_token: Option<String>,
        /// Unix timestamp. `0` means "unknown".
        #[serde(default)]
        expires_at: i64,
    },
}

impl AccountRecord {
    pub fn offline(username: &str) -> Self {
        let username = username.trim().to_string();
        Self::Offline {
            id: format!("offline:{}", username.to_lowercase()),
            username,
        }
    }

    pub fn id(&self) -> String {
        match self {
            AccountRecord::Offline { id, .. } => id.clone(),
            AccountRecord::Microsoft { id, .. } => id.clone(),
        }
    }

    pub fn username(&self) -> &str {
        match self {
            AccountRecord::Offline { username, .. } => username,
            AccountRecord::Microsoft { username, .. } => username,
        }
    }

    /// Key used for the avatar service: uuid for Microsoft, name for offline.
    pub fn skin_key(&self) -> String {
        match self {
            AccountRecord::Offline { username, .. } => username.clone(),
            AccountRecord::Microsoft { uuid, username, .. } => {
                if uuid.is_empty() {
                    username.clone()
                } else {
                    uuid.clone()
                }
            }
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            AccountRecord::Offline { .. } => "Offline",
            AccountRecord::Microsoft { .. } => "Microsoft",
        }
    }

    pub fn is_expired(&self) -> bool {
        match self {
            AccountRecord::Offline { .. } => false,
            AccountRecord::Microsoft { expires_at, .. } => {
                *expires_at != 0 && *expires_at <= now_unix()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Device-code flow
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct MsaToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct MinecraftProfile {
    pub name: String,
    pub uuid: String,
}

fn http() -> anyhow::Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(USER_AGENT)
        .build()?)
}

/// Step 1 — ask Microsoft for a code the user types into a browser.
pub fn start_device_code(client_id: &str) -> anyhow::Result<DeviceCode> {
    if client_id.trim().is_empty() {
        anyhow::bail!("No Microsoft client id configured (Preferences → Microsoft client id).");
    }

    let response = http()?
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", client_id), ("scope", MSA_SCOPE)])
        .send()?
        .error_for_status()?
        .json::<serde_json::Value>()?;

    let field = |key: &str| -> anyhow::Result<String> {
        response
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("device code response missing `{key}`"))
    };

    Ok(DeviceCode {
        device_code: field("device_code")?,
        user_code: field("user_code")?,
        verification_uri: response
            .get("verification_uri")
            .and_then(|v| v.as_str())
            .unwrap_or("https://microsoft.com/link")
            .to_string(),
        interval: response.get("interval").and_then(|v| v.as_u64()).unwrap_or(5),
        expires_in: response.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(900),
    })
}

/// Step 2 — block until the user finishes in the browser.
///
/// Returns `Err` if the user declines or the code expires. Cancellation is not
/// supported yet; the caller simply drops the worker thread's result.
pub fn poll_device_code(client_id: &str, code: &DeviceCode) -> anyhow::Result<MsaToken> {
    let client = http()?;
    let mut interval = code.interval.max(1);
    let deadline = SystemTime::now() + Duration::from_secs(code.expires_in);

    loop {
        if SystemTime::now() > deadline {
            anyhow::bail!("The sign-in code expired. Please try again.");
        }

        std::thread::sleep(Duration::from_secs(interval));

        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("client_id", client_id),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", code.device_code.as_str()),
            ])
            .send()?;

        let status = response.status();
        let body = response.json::<serde_json::Value>()?;

        if status.is_success() {
            let access_token = body
                .get("access_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("token response missing access_token"))?
                .to_string();

            return Ok(MsaToken {
                access_token,
                refresh_token: body
                    .get("refresh_token")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                expires_in: body.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600),
            });
        }

        match body.get("error").and_then(|v| v.as_str()).unwrap_or("") {
            "authorization_pending" => continue,
            "slow_down" => {
                interval += 5;
                continue;
            }
            "expired_token" => anyhow::bail!("The sign-in code expired. Please try again."),
            "authorization_declined" => anyhow::bail!("Sign-in was declined."),
            other => {
                let description = body
                    .get("error_description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                anyhow::bail!("Microsoft sign-in failed ({other}): {description}");
            }
        }
    }
}

/// Steps 3–6 — exchange the MSA token for a Minecraft profile.
pub fn msa_to_minecraft(msa: &MsaToken) -> anyhow::Result<MinecraftProfile> {
    let client = http()?;

    // 3. Xbox Live
    let xbl = client
        .post(XBL_URL)
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", msa.access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .send()?
        .error_for_status()?
        .json::<serde_json::Value>()?;

    let xbl_token = xbl
        .get("Token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Xbox Live response missing Token"))?
        .to_string();

    // 4. XSTS
    let xsts_response = client
        .post(XSTS_URL)
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send()?;

    let xsts_status = xsts_response.status();
    let xsts = xsts_response.json::<serde_json::Value>()?;

    if !xsts_status.is_success() {
        // XErr codes are documented at https://wiki.vg/Microsoft_Authentication_Scheme
        let xerr = xsts
            .get("XErr")
            .and_then(|v| v.as_u64())
            .unwrap_or_default();
        let hint = match xerr {
            2148916233 => "This Microsoft account has no Xbox profile. Sign in at xbox.com once.",
            2148916235 => "Xbox Live is not available in this country.",
            2148916238 => "This account is a child account and needs an adult in a family group.",
            _ => "Xbox Live authorization failed.",
        };
        anyhow::bail!("{hint} (XErr {xerr})");
    }

    let xsts_token = xsts
        .get("Token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("XSTS response missing Token"))?
        .to_string();

    let uhs = xsts
        .get("DisplayClaims")
        .and_then(|v| v.get("xui"))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("uhs"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("XSTS response missing user hash"))?
        .to_string();

    // 5. Minecraft services
    let mc = client
        .post(MC_LOGIN_URL)
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{xsts_token}"),
        }))
        .send()?
        .error_for_status()?
        .json::<serde_json::Value>()?;

    let mc_token = mc
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Minecraft login response missing access_token"))?
        .to_string();

    // 6. Profile — also confirms the account owns the game.
    let profile_response = client
        .get(MC_PROFILE_URL)
        .bearer_auth(&mc_token)
        .send()?;

    if profile_response.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("This account does not own Minecraft: Java Edition.");
    }

    let profile = profile_response
        .error_for_status()?
        .json::<serde_json::Value>()?;

    Ok(MinecraftProfile {
        name: profile
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Player")
            .to_string(),
        uuid: profile
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

/// Convenience wrapper: full chain, returning a stored record.
pub fn microsoft_login(client_id: &str, code: &DeviceCode) -> anyhow::Result<AccountRecord> {
    let msa = poll_device_code(client_id, code)?;
    let profile = msa_to_minecraft(&msa)?;

    Ok(AccountRecord::Microsoft {
        id: format!("ms:{}", profile.uuid),
        username: profile.name,
        uuid: profile.uuid,
        access_token: msa.access_token,
        refresh_token: msa.refresh_token,
        expires_at: now_unix() + msa.expires_in as i64,
    })
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
