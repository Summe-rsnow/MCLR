//! Microsoft authentication flow (OAuth device code, Xbox Live, XSTS, Minecraft login, profile).

use crate::error::AuthError;
use crate::offline::default_skin_model;
use crate::skin::{CapeView, crop_skin_head};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeEvent {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct MsAuthEndpoints {
    pub device_code: String,
    pub token: String,
    pub xbox_live: String,
    pub xsts: String,
    pub minecraft_login: String,
    pub mc_store: String,
    pub mc_profile: String,
}

impl Default for MsAuthEndpoints {
    fn default() -> Self {
        Self {
            device_code: "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode"
                .into(),
            token: "https://login.microsoftonline.com/consumers/oauth2/v2.0/token".into(),
            xbox_live: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            minecraft_login: "https://api.minecraftservices.com/authentication/login_with_xbox"
                .into(),
            mc_store: "https://api.minecraftservices.com/entitlements/mcstore".into(),
            mc_profile: "https://api.minecraftservices.com/minecraft/profile".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
struct TokenSuccessResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    _expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: String,
}

#[derive(Debug, Serialize)]
struct XboxLiveAuthRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XboxLiveAuthProperties<'a>,
    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,
    #[serde(rename = "TokenType")]
    token_type: &'a str,
}

#[derive(Debug, Serialize)]
struct XboxLiveAuthProperties<'a> {
    #[serde(rename = "AuthMethod")]
    auth_method: &'a str,
    #[serde(rename = "SiteName")]
    site_name: &'a str,
    #[serde(rename = "RpsTicket")]
    rps_ticket: &'a str,
}

#[derive(Debug, Serialize)]
struct XstsAuthRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XstsAuthProperties<'a>,
    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,
    #[serde(rename = "TokenType")]
    token_type: &'a str,
}

#[derive(Debug, Serialize)]
struct XstsAuthProperties<'a> {
    #[serde(rename = "SandboxId")]
    sandbox_id: &'a str,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<&'a str>,
}

#[derive(Debug, Deserialize)]
struct XboxTokenResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: Option<XboxDisplayClaims>,
}

#[derive(Debug, Deserialize)]
struct XboxDisplayClaims {
    #[serde(default)]
    xui: Vec<XboxUserHash>,
}

#[derive(Debug, Deserialize)]
struct XboxUserHash {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct McStoreResponse {
    #[serde(default)]
    items: Vec<McStoreItem>,
}

#[derive(Debug, Deserialize)]
struct McStoreItem {
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct McProfileResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<McProfileSkin>,
    #[serde(default)]
    pub capes: Vec<McProfileCape>,
}

#[derive(Debug, Deserialize)]
pub struct McProfileSkin {
    pub id: String,
    #[serde(default)]
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub variant: String,
}

#[derive(Debug, Deserialize)]
pub struct McProfileCape {
    pub id: String,
    #[serde(default)]
    pub state: String,
    pub url: String,
    pub alias: String,
}

#[derive(Debug, Clone)]
pub struct MicrosoftLoginResult {
    pub uuid: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub skin_head_data_url: Option<String>,
    pub capes: Vec<CapeView>,
    pub default_skin_model: String,
}

/// Well-known public Azure Client ID used for Minecraft device-code authentication (PCL / PrismLauncher standard).
///
/// Can be overridden at runtime via `MCLR_MS_CLIENT_ID` in environment variables or `.env`.
pub const DEFAULT_MS_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

#[derive(Debug)]
pub struct MicrosoftAuthService {
    client: Client,
    endpoints: MsAuthEndpoints,
    client_id: String,
}

impl MicrosoftAuthService {
    pub fn new(
        client: Client,
        endpoints: MsAuthEndpoints,
        custom_client_id: Option<String>,
    ) -> Result<Self, AuthError> {
        let client_id = match custom_client_id {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            Some(_) => return Err(AuthError::NotConfigured { what: "microsoft" }),
            None => match mclr_core::secrets::get_secret("MCLR_MS_CLIENT_ID") {
                Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                _ => DEFAULT_MS_CLIENT_ID.to_string(),
            },
        };

        Ok(Self {
            client,
            endpoints,
            client_id,
        })
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Step 1: Request device code from Microsoft OAuth
    pub async fn get_device_code(&self) -> Result<DeviceCodeResponse, AuthError> {
        let mut params = HashMap::new();
        params.insert("client_id", self.client_id.as_str());
        params.insert("scope", "XboxLive.signin offline_access");

        let res = self
            .client
            .post(&self.endpoints.device_code)
            .form(&params)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::Http(format!(
                "device code request failed {status}: {body}"
            )));
        }

        let resp: DeviceCodeResponse = res.json().await?;
        Ok(resp)
    }

    /// Step 2: Poll token endpoint until user authorizes or expires
    pub async fn poll_token(
        &self,
        device_code: &str,
        mut interval: u64,
        control: Option<&mclr_core::tasks::TaskControl>,
    ) -> Result<(String, String), AuthError> {
        if interval == 0 {
            interval = 5;
        }

        loop {
            if let Some(ctrl) = control
                && ctrl.checkpoint().await.is_err()
            {
                return Err(AuthError::Other("Task was canceled".into()));
            }

            sleep(Duration::from_secs(interval)).await;

            if let Some(ctrl) = control
                && ctrl.checkpoint().await.is_err()
            {
                return Err(AuthError::Other("Task was canceled".into()));
            }

            let mut params = HashMap::new();
            params.insert("client_id", self.client_id.as_str());
            params.insert("grant_type", "urn:ietf:params:oauth:grant-type:device_code");
            params.insert("device_code", device_code);

            let res = self
                .client
                .post(&self.endpoints.token)
                .form(&params)
                .send()
                .await?;

            if res.status().is_success() {
                let token_resp: TokenSuccessResponse = res.json().await?;
                let refresh = token_resp.refresh_token.unwrap_or_default();
                return Ok((token_resp.access_token, refresh));
            }

            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            if let Ok(err_resp) = serde_json::from_str::<TokenErrorResponse>(&body) {
                match err_resp.error.as_str() {
                    "authorization_pending" => {
                        continue;
                    }
                    "slow_down" => {
                        interval += 5;
                        continue;
                    }
                    "expired_token" => {
                        return Err(AuthError::MsExpiredToken);
                    }
                    "access_denied" => {
                        return Err(AuthError::MsAccessDenied);
                    }
                    other => {
                        return Err(AuthError::Http(format!(
                            "token poll error {other}: {}",
                            err_resp.error_description
                        )));
                    }
                }
            } else {
                return Err(AuthError::Http(format!(
                    "token poll request failed {status}: {body}"
                )));
            }
        }
    }

    /// Step 3: Xbox Live authentication
    pub async fn authenticate_xbox_live(&self, ms_access_token: &str) -> Result<String, AuthError> {
        let ticket = format!("d={ms_access_token}");
        let req = XboxLiveAuthRequest {
            properties: XboxLiveAuthProperties {
                auth_method: "RPS",
                site_name: "user.auth.xboxlive.com",
                rps_ticket: &ticket,
            },
            relying_party: "http://auth.xboxlive.com",
            token_type: "JWT",
        };

        let res = self
            .client
            .post(&self.endpoints.xbox_live)
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::Http(format!(
                "Xbox Live authenticate failed {status}: {body}"
            )));
        }

        let resp: XboxTokenResponse = res.json().await?;
        Ok(resp.token)
    }

    /// Step 4: XSTS authorization with XErr error parsing
    pub async fn authorize_xsts(&self, xbl_token: &str) -> Result<(String, String), AuthError> {
        let req = XstsAuthRequest {
            properties: XstsAuthProperties {
                sandbox_id: "RETAIL",
                user_tokens: vec![xbl_token],
            },
            relying_party: "rp://api.minecraftservices.com/",
            token_type: "JWT",
        };

        let res = self
            .client
            .post(&self.endpoints.xsts)
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body)
                && let Some(xerr) = val.get("XErr").and_then(|v| v.as_i64())
            {
                match xerr {
                    2148916227 => return Err(AuthError::XstsBanned),
                    2148916233 => return Err(AuthError::XstsNoXboxAccount),
                    2148916235 => return Err(AuthError::XstsRegionBlocked),
                    2148916238 => return Err(AuthError::XstsMinor),
                    _ => {}
                }
            }

            return Err(AuthError::Http(format!(
                "XSTS authorization failed {status}: {body}"
            )));
        }

        let resp: XboxTokenResponse = res.json().await?;
        let user_hash = resp
            .display_claims
            .and_then(|c| c.xui.into_iter().next())
            .map(|x| x.uhs)
            .ok_or_else(|| AuthError::Http("XSTS response missing user hash".to_string()))?;

        Ok((resp.token, user_hash))
    }

    /// Step 5: Minecraft login with Xbox token and user hash
    pub async fn login_minecraft(
        &self,
        xsts_token: &str,
        user_hash: &str,
    ) -> Result<(String, i64), AuthError> {
        let identity_token = format!("XBL3.0 x={user_hash};{xsts_token}");
        let body = serde_json::json!({
            "identityToken": identity_token
        });

        let res = self
            .client
            .post(&self.endpoints.minecraft_login)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(AuthError::Http(format!(
                "Minecraft login failed {status}: {text}"
            )));
        }

        let resp: MinecraftLoginResponse = res.json().await?;
        let expires_at = Utc::now().timestamp() + resp.expires_in as i64;
        Ok((resp.access_token, expires_at))
    }

    /// Step 6: Verify entitlements and retrieve Minecraft profile
    pub async fn get_profile(&self, mc_access_token: &str) -> Result<McProfileResponse, AuthError> {
        // 1. Check entitlements
        let store_res = self
            .client
            .get(&self.endpoints.mc_store)
            .bearer_auth(mc_access_token)
            .send()
            .await?;

        if store_res.status().is_success() {
            let store: McStoreResponse = store_res.json().await?;
            let owns_mc = store
                .items
                .iter()
                .any(|item| item.name == "product_minecraft" || item.name == "game_minecraft");
            if !owns_mc {
                return Err(AuthError::MsNoProfile);
            }
        } else {
            return Err(AuthError::MsNoProfile);
        }

        // 2. Fetch profile
        let prof_res = self
            .client
            .get(&self.endpoints.mc_profile)
            .bearer_auth(mc_access_token)
            .send()
            .await?;

        if prof_res.status().as_u16() == 404 {
            return Err(AuthError::MsNoProfile);
        }

        if !prof_res.status().is_success() {
            let status = prof_res.status();
            let body = prof_res.text().await.unwrap_or_default();
            return Err(AuthError::Http(format!(
                "Failed to fetch profile {status}: {body}"
            )));
        }

        let profile: McProfileResponse = prof_res.json().await?;
        Ok(profile)
    }

    /// Completes the 6-step login flow after MS access token is acquired.
    pub async fn complete_flow(
        &self,
        ms_access_token: &str,
        refresh_token: &str,
    ) -> Result<MicrosoftLoginResult, AuthError> {
        let xbl_token = self.authenticate_xbox_live(ms_access_token).await?;
        let (xsts_token, user_hash) = self.authorize_xsts(&xbl_token).await?;
        let (mc_access_token, expires_at) = self.login_minecraft(&xsts_token, &user_hash).await?;
        let profile = self.get_profile(&mc_access_token).await?;

        let model = default_skin_model(&profile.id).to_string();

        // Download skin if available and generate cropped head
        let mut skin_head_data_url = None;
        if let Some(active_skin) = profile.skins.iter().find(|s| s.state == "ACTIVE")
            && let Ok(skin_res) = self.client.get(&active_skin.url).send().await
            && let Ok(skin_bytes) = skin_res.bytes().await
            && let Ok(data_url) = crop_skin_head(&skin_bytes, &profile.id)
        {
            skin_head_data_url = Some(data_url);
        }

        let capes = profile
            .capes
            .into_iter()
            .map(|c| CapeView {
                id: c.id,
                alias: c.alias,
                url: c.url,
                active: c.state == "ACTIVE",
            })
            .collect();

        Ok(MicrosoftLoginResult {
            uuid: profile.id,
            username: profile.name,
            access_token: mc_access_token,
            refresh_token: refresh_token.to_string(),
            expires_at,
            skin_head_data_url,
            capes,
            default_skin_model: model,
        })
    }

    /// Refreshes OAuth token using refresh_token, then executes Xbox Live -> XSTS -> Minecraft login.
    pub async fn refresh(&self, refresh_token: &str) -> Result<MicrosoftLoginResult, AuthError> {
        let mut params = HashMap::new();
        params.insert("client_id", self.client_id.as_str());
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);
        params.insert("scope", "XboxLive.signin offline_access");

        let res = self
            .client
            .post(&self.endpoints.token)
            .form(&params)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::Http(format!(
                "refresh token failed {status}: {body}"
            )));
        }

        let token_resp: TokenSuccessResponse = res.json().await?;
        let new_refresh = token_resp
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string());

        self.complete_flow(&token_resp.access_token, &new_refresh)
            .await
    }
}
