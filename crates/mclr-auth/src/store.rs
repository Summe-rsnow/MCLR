//! Encrypted account store with AES-256-GCM and DPAPI master key.

use crate::dpapi::{protect_bytes, unprotect_bytes};
use crate::error::AuthError;
use crate::microsoft::{MicrosoftAuthService, MicrosoftLoginResult, MsAuthEndpoints};
use crate::offline::{OfflineUuidMode, compute_offline_uuid, default_skin_model};
use crate::skin::{CapeView, default_head_data_url};
use crate::yggdrasil::{
    AuthlibProfileView, YggdrasilProfile, authenticate, discover_api_location, fetch_profile_head,
    fetch_server_metadata, refresh as yggdrasil_refresh,
};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use async_trait::async_trait;
use mclr_core::account_session::{AccountKind, AccountSessionProvider, ResolvedAccountSession};
use mclr_core::{AppError, paths};
use rand::Rng;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

const STORE_MAGIC: &[u8; 4] = b"MCLR";
const STORE_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub kind: AccountKind,
    pub username: String,
    pub uuid: String,
    pub selected: bool,
    pub server: Option<String>,
    pub server_name: Option<String>,
    pub skin_head_data_url: Option<String>,
    pub capes: Vec<CapeView>,
    pub default_skin_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AuthlibLoginResponse {
    pub account: Option<AccountView>,
    pub pending_session_id: Option<String>,
    pub profiles: Vec<AuthlibProfileView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StoredAccount {
    Microsoft {
        id: String,
        uuid: String,
        username: String,
        access_token: String,
        refresh_token: String,
        expires_at: i64,
        #[serde(default)]
        skin_head_data_url: Option<String>,
        #[serde(default)]
        capes: Vec<CapeView>,
        default_skin_model: String,
    },
    Offline {
        id: String,
        username: String,
        uuid: String,
        uuid_mode: OfflineUuidMode,
        #[serde(default)]
        skin_head_data_url: Option<String>,
        default_skin_model: String,
    },
    Authlib {
        id: String,
        server: String,
        server_name: Option<String>,
        username: String,
        uuid: String,
        access_token: String,
        client_token: String,
        #[serde(default)]
        profiles: Vec<AuthlibProfileView>,
        #[serde(default)]
        skin_head_data_url: Option<String>,
        default_skin_model: String,
    },
    YggdrasilConnect {
        id: String,
        discovery_url: String,
        uuid: String,
        username: String,
        access_token: String,
        refresh_token: String,
        id_token: String,
        expires_at: i64,
        #[serde(default)]
        skin_head_data_url: Option<String>,
        #[serde(default)]
        capes: Vec<CapeView>,
        default_skin_model: String,
    },
}

impl StoredAccount {
    pub fn id(&self) -> &str {
        match self {
            StoredAccount::Microsoft { id, .. } => id,
            StoredAccount::Offline { id, .. } => id,
            StoredAccount::Authlib { id, .. } => id,
            StoredAccount::YggdrasilConnect { id, .. } => id,
        }
    }

    pub fn to_view(&self, is_selected: bool) -> AccountView {
        match self {
            StoredAccount::Microsoft {
                id,
                uuid,
                username,
                skin_head_data_url,
                capes,
                default_skin_model,
                ..
            } => AccountView {
                id: id.clone(),
                kind: AccountKind::Microsoft,
                username: username.clone(),
                uuid: uuid.clone(),
                selected: is_selected,
                server: None,
                server_name: None,
                skin_head_data_url: skin_head_data_url.clone(),
                capes: capes.clone(),
                default_skin_model: default_skin_model.clone(),
            },
            StoredAccount::Offline {
                id,
                username,
                uuid,
                skin_head_data_url,
                default_skin_model,
                ..
            } => {
                let head_url = skin_head_data_url
                    .clone()
                    .or_else(|| Some(default_head_data_url(default_skin_model)));
                AccountView {
                    id: id.clone(),
                    kind: AccountKind::Offline,
                    username: username.clone(),
                    uuid: uuid.clone(),
                    selected: is_selected,
                    server: None,
                    server_name: None,
                    skin_head_data_url: head_url,
                    capes: vec![],
                    default_skin_model: default_skin_model.clone(),
                }
            }
            StoredAccount::Authlib {
                id,
                server,
                server_name,
                username,
                uuid,
                skin_head_data_url,
                default_skin_model,
                ..
            } => AccountView {
                id: id.clone(),
                kind: AccountKind::Authlib,
                username: username.clone(),
                uuid: uuid.clone(),
                selected: is_selected,
                server: Some(server.clone()),
                server_name: server_name.clone(),
                skin_head_data_url: skin_head_data_url.clone(),
                capes: vec![],
                default_skin_model: default_skin_model.clone(),
            },
            StoredAccount::YggdrasilConnect {
                id,
                discovery_url,
                uuid,
                username,
                skin_head_data_url,
                capes,
                default_skin_model,
                ..
            } => AccountView {
                id: id.clone(),
                kind: AccountKind::YggdrasilConnect,
                username: username.clone(),
                uuid: uuid.clone(),
                selected: is_selected,
                server: Some(discovery_url.clone()),
                server_name: None,
                skin_head_data_url: skin_head_data_url.clone(),
                capes: capes.clone(),
                default_skin_model: default_skin_model.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredData {
    pub last_used: Option<String>,
    pub accounts: Vec<StoredAccount>,
}

#[derive(Debug, Clone)]
pub struct PendingAuthlibSession {
    pub server: String,
    pub server_name: Option<String>,
    pub access_token: String,
    pub client_token: String,
    pub profiles: Vec<AuthlibProfileView>,
}

pub struct AccountStoreInner {
    pub data: StoredData,
    pub master_key: [u8; 32],
    pub pending_sessions: HashMap<String, PendingAuthlibSession>,
}

#[derive(Clone)]
pub struct AccountStore {
    inner: Arc<RwLock<AccountStoreInner>>,
}

impl AccountStore {
    /// Opens or initializes the encrypted account store at `%APPDATA%/MCLR/accounts.enc`.
    pub fn open() -> Result<Self, AuthError> {
        let master_key_path = paths::master_key_file();
        let master_key = Self::load_or_create_master_key(&master_key_path)?;

        let accounts_path = paths::accounts_file();
        let data = if accounts_path.exists() {
            Self::read_encrypted_store(&accounts_path, &master_key)?
        } else {
            StoredData::default()
        };

        Ok(Self {
            inner: Arc::new(RwLock::new(AccountStoreInner {
                data,
                master_key,
                pending_sessions: HashMap::new(),
            })),
        })
    }

    fn load_or_create_master_key(path: &PathBuf) -> Result<[u8; 32], AuthError> {
        if path.exists() {
            let enc_bytes = fs::read(path)?;
            let raw_bytes = unprotect_bytes(&enc_bytes)?;
            if raw_bytes.len() != 32 {
                return Err(AuthError::StoreCorrupted(
                    "master key has invalid length".into(),
                ));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&raw_bytes);
            Ok(key)
        } else {
            let mut key = [0u8; 32];
            rand::thread_rng().fill(&mut key);
            let enc_bytes = protect_bytes(&key)?;
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(path, enc_bytes)?;
            Ok(key)
        }
    }

    fn read_encrypted_store(
        path: &PathBuf,
        master_key: &[u8; 32],
    ) -> Result<StoredData, AuthError> {
        let file_bytes = fs::read(path)?;
        if file_bytes.len() < 17 {
            return Err(AuthError::StoreCorrupted("store file is too short".into()));
        }

        if &file_bytes[0..4] != STORE_MAGIC {
            return Err(AuthError::StoreCorrupted("invalid store file magic".into()));
        }

        if file_bytes[4] != STORE_VERSION {
            return Err(AuthError::StoreCorrupted(format!(
                "unsupported store version: {}",
                file_bytes[4]
            )));
        }

        let nonce = Nonce::from_slice(&file_bytes[5..17]);
        let ciphertext = &file_bytes[17..];

        let cipher =
            Aes256Gcm::new_from_slice(master_key).map_err(|e| AuthError::Crypto(e.to_string()))?;
        let decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AuthError::StoreCorrupted(format!("decryption failed: {e}")))?;

        let data: StoredData = serde_json::from_slice(&decrypted)
            .map_err(|e| AuthError::StoreCorrupted(format!("invalid JSON in store: {e}")))?;
        Ok(data)
    }

    fn save_encrypted_store(
        path: &PathBuf,
        data: &StoredData,
        master_key: &[u8; 32],
    ) -> Result<(), AuthError> {
        let json_bytes = serde_json::to_vec(data)?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher =
            Aes256Gcm::new_from_slice(master_key).map_err(|e| AuthError::Crypto(e.to_string()))?;
        let ciphertext = cipher
            .encrypt(nonce, json_bytes.as_ref())
            .map_err(|e| AuthError::Crypto(e.to_string()))?;

        let mut output = Vec::with_capacity(4 + 1 + 12 + ciphertext.len());
        output.extend_from_slice(STORE_MAGIC);
        output.push(STORE_VERSION);
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Atomic write
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, output)?;
        fs::rename(&tmp_path, path)?;

        Ok(())
    }

    async fn persist(&self) -> Result<(), AuthError> {
        let inner = self.inner.read().await;
        Self::save_encrypted_store(&paths::accounts_file(), &inner.data, &inner.master_key)
    }

    pub async fn list_accounts(&self) -> Vec<AccountView> {
        let inner = self.inner.read().await;
        let last_used = inner.data.last_used.as_deref();
        inner
            .data
            .accounts
            .iter()
            .map(|a| a.to_view(last_used == Some(a.id())))
            .collect()
    }

    pub async fn select_account(&self, id: &str) -> Result<Vec<AccountView>, AuthError> {
        {
            let mut inner = self.inner.write().await;
            if !inner.data.accounts.iter().any(|a| a.id() == id) {
                return Err(AuthError::AccountNotFound(id.to_string()));
            }
            inner.data.last_used = Some(id.to_string());
        }
        self.persist().await?;
        Ok(self.list_accounts().await)
    }

    /// Repairs a dangling `last_used` pointer.
    ///
    /// The account store on disk can be edited or truncated while the launcher is
    /// not running, leaving `last_used` pointing at an account that no longer
    /// exists. In that case the whole list would report `selected == false` and
    /// the home page would show no active account. Fall back to the first
    /// available account (or `None` when the list is empty) and persist it.
    pub async fn ensure_valid_selection(&self) -> Result<(), AuthError> {
        let changed = {
            let mut inner = self.inner.write().await;
            let valid = inner
                .data
                .last_used
                .as_deref()
                .is_some_and(|id| inner.data.accounts.iter().any(|a| a.id() == id));
            if valid {
                false
            } else {
                let next = inner.data.accounts.first().map(|a| a.id().to_string());
                if inner.data.last_used == next {
                    false
                } else {
                    inner.data.last_used = next;
                    true
                }
            }
        };
        if changed {
            self.persist().await?;
        }
        Ok(())
    }

    /// Lists accounts after repairing a dangling selection pointer.
    pub async fn list_accounts_healed(&self) -> Result<Vec<AccountView>, AuthError> {
        self.ensure_valid_selection().await?;
        Ok(self.list_accounts().await)
    }

    pub async fn remove_account(&self, id: &str) -> Result<Vec<AccountView>, AuthError> {
        {
            let mut inner = self.inner.write().await;
            let initial_len = inner.data.accounts.len();
            inner.data.accounts.retain(|a| a.id() != id);
            if inner.data.accounts.len() == initial_len {
                return Err(AuthError::AccountNotFound(id.to_string()));
            }
            if inner.data.last_used.as_deref() == Some(id) {
                inner.data.last_used = inner.data.accounts.first().map(|a| a.id().to_string());
            }
        }
        self.persist().await?;
        Ok(self.list_accounts().await)
    }

    pub async fn add_offline(
        &self,
        username: &str,
        uuid_mode: OfflineUuidMode,
        custom_uuid: Option<&str>,
    ) -> Result<AccountView, AuthError> {
        let trimmed_name = username.trim();
        if trimmed_name.is_empty() {
            return Err(AuthError::Other("username cannot be empty".into()));
        }

        let uuid = compute_offline_uuid(trimmed_name, uuid_mode, custom_uuid)?;
        let model = default_skin_model(&uuid).to_string();
        let id = Uuid::new_v4().to_string();

        let account = StoredAccount::Offline {
            id: id.clone(),
            username: trimmed_name.to_string(),
            uuid,
            uuid_mode,
            skin_head_data_url: None,
            default_skin_model: model,
        };

        let view = account.to_view(true);
        {
            let mut inner = self.inner.write().await;
            inner.data.accounts.push(account);
            inner.data.last_used = Some(id);
        }
        self.persist().await?;
        Ok(view)
    }
    pub async fn rename_offline(&self, id: &str, username: &str) -> Result<AccountView, AuthError> {
        let trimmed_name = username.trim();
        if trimmed_name.is_empty() {
            return Err(AuthError::Other("username cannot be empty".into()));
        }

        let mut inner = self.inner.write().await;
        let is_selected = inner.data.last_used.as_deref() == Some(id);

        // Find the account and ensure it's offline
        let account = inner
            .data
            .accounts
            .iter_mut()
            .find(|a| a.id() == id)
            .ok_or_else(|| AuthError::AccountNotFound(id.to_string()))?;

        let updated_view = match account {
            StoredAccount::Offline {
                username: acc_name,
                uuid,
                uuid_mode,
                default_skin_model: acc_model,
                ..
            } => {
                // Recompute uuid unless mode is custom
                if *uuid_mode != OfflineUuidMode::Custom {
                    *uuid = compute_offline_uuid(trimmed_name, *uuid_mode, None)?;
                    *acc_model = default_skin_model(uuid).to_string();
                }
                *acc_name = trimmed_name.to_string();
                account.to_view(is_selected)
            }
            _ => {
                return Err(AuthError::Other("account is not an offline account".into()));
            }
        };

        drop(inner);
        self.persist().await?;
        Ok(updated_view)
    }

    pub async fn add_microsoft(
        &self,
        login: MicrosoftLoginResult,
    ) -> Result<AccountView, AuthError> {
        let mut inner = self.inner.write().await;

        // If account with this UUID already exists, update tokens & profile
        let existing_id = inner
            .data
            .accounts
            .iter()
            .find(|a| match a {
                StoredAccount::Microsoft { uuid, .. } => uuid == &login.uuid,
                _ => false,
            })
            .map(|a| a.id().to_string());

        let id = existing_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let account = StoredAccount::Microsoft {
            id: id.clone(),
            uuid: login.uuid,
            username: login.username,
            access_token: login.access_token,
            refresh_token: login.refresh_token,
            expires_at: login.expires_at,
            skin_head_data_url: login.skin_head_data_url,
            capes: login.capes,
            default_skin_model: login.default_skin_model,
        };

        let view = account.to_view(true);

        if let Some(pos) = inner.data.accounts.iter().position(|a| a.id() == id) {
            inner.data.accounts[pos] = account;
        } else {
            inner.data.accounts.push(account);
        }
        inner.data.last_used = Some(id);

        drop(inner);
        self.persist().await?;
        Ok(view)
    }

    pub async fn add_authlib(
        &self,
        client: &reqwest::Client,
        server: &str,
        username: &str,
        password: &str,
    ) -> Result<AuthlibLoginResponse, AuthError> {
        let api_root = discover_api_location(client, server).await?;
        let meta = fetch_server_metadata(client, &api_root).await?;
        let server_name = meta.meta.and_then(|m| m.server_name);

        let auth_res = authenticate(client, &api_root, username, password).await?;

        let profiles_view: Vec<AuthlibProfileView> = auth_res
            .available_profiles
            .iter()
            .map(|p| AuthlibProfileView {
                id: p.id.clone(),
                name: p.name.clone(),
            })
            .collect();

        // If only 1 profile or auto-selected
        if profiles_view.len() <= 1 {
            let selected = auth_res
                .selected_profile
                .or_else(|| auth_res.available_profiles.into_iter().next())
                .ok_or_else(|| {
                    AuthError::Other("no selectable profile returned by authlib server".into())
                })?;

            let model = default_skin_model(&selected.id).to_string();
            let id = Uuid::new_v4().to_string();
            let skin_head = fetch_profile_head(client, &api_root, &selected.id).await;

            let account = StoredAccount::Authlib {
                id: id.clone(),
                server: api_root,
                server_name,
                username: selected.name,
                uuid: selected.id,
                access_token: auth_res.access_token,
                client_token: auth_res.client_token,
                profiles: profiles_view,
                skin_head_data_url: skin_head,
                default_skin_model: model,
            };

            let view = account.to_view(true);
            {
                let mut inner = self.inner.write().await;
                inner.data.accounts.push(account);
                inner.data.last_used = Some(id);
            }
            self.persist().await?;

            Ok(AuthlibLoginResponse {
                account: Some(view),
                pending_session_id: None,
                profiles: vec![],
            })
        } else {
            // Multiple profiles -> store pending session for selection
            let session_id = Uuid::new_v4().to_string();
            let pending = PendingAuthlibSession {
                server: api_root,
                server_name,
                access_token: auth_res.access_token,
                client_token: auth_res.client_token,
                profiles: profiles_view.clone(),
            };

            let mut inner = self.inner.write().await;
            inner.pending_sessions.insert(session_id.clone(), pending);

            Ok(AuthlibLoginResponse {
                account: None,
                pending_session_id: Some(session_id),
                profiles: profiles_view,
            })
        }
    }

    pub async fn select_authlib_profile(
        &self,
        client: &reqwest::Client,
        pending_session_id: &str,
        profile_id: &str,
    ) -> Result<AccountView, AuthError> {
        let pending = {
            let mut inner = self.inner.write().await;
            inner
                .pending_sessions
                .remove(pending_session_id)
                .ok_or_else(|| AuthError::Other("pending session expired or not found".into()))?
        };

        let chosen_profile = pending
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| AuthError::Other("selected profile not found in session".into()))?;

        // Refresh / bind selected profile
        let ygg_profile = YggdrasilProfile {
            id: chosen_profile.id.clone(),
            name: chosen_profile.name.clone(),
        };

        let refresh_res = yggdrasil_refresh(
            client,
            &pending.server,
            &pending.access_token,
            &pending.client_token,
            Some(&ygg_profile),
        )
        .await?;

        let model = default_skin_model(&chosen_profile.id).to_string();
        let id = Uuid::new_v4().to_string();

        let skin_head = fetch_profile_head(client, &pending.server, &chosen_profile.id).await;

        let account = StoredAccount::Authlib {
            id: id.clone(),
            server: pending.server,
            server_name: pending.server_name,
            username: chosen_profile.name.clone(),
            uuid: chosen_profile.id.clone(),
            access_token: refresh_res.access_token,
            client_token: refresh_res.client_token,
            profiles: pending.profiles,
            skin_head_data_url: skin_head,
            default_skin_model: model,
        };

        let view = account.to_view(true);
        {
            let mut inner = self.inner.write().await;
            inner.data.accounts.push(account);
            inner.data.last_used = Some(id);
        }
        self.persist().await?;
        Ok(view)
    }

    pub async fn refresh_account(
        &self,
        client: &reqwest::Client,
        id: &str,
    ) -> Result<AccountView, AuthError> {
        let account = {
            let inner = self.inner.read().await;
            inner
                .data
                .accounts
                .iter()
                .find(|a| a.id() == id)
                .cloned()
                .ok_or_else(|| AuthError::AccountNotFound(id.to_string()))?
        };

        match account {
            StoredAccount::Microsoft { refresh_token, .. } => {
                let ms_service =
                    MicrosoftAuthService::new(client.clone(), MsAuthEndpoints::default(), None)?;
                let login = ms_service.refresh(&refresh_token).await?;
                self.add_microsoft(login).await
            }
            StoredAccount::Authlib {
                server,
                access_token,
                client_token,
                uuid,
                username,
                ..
            } => {
                let ygg_profile = YggdrasilProfile {
                    id: uuid.clone(),
                    name: username.clone(),
                };
                let res = yggdrasil_refresh(
                    client,
                    &server,
                    &access_token,
                    &client_token,
                    Some(&ygg_profile),
                )
                .await?;

                let is_selected = {
                    let inner = self.inner.read().await;
                    inner.data.last_used.as_deref() == Some(id)
                };

                let mut inner = self.inner.write().await;
                if let Some(StoredAccount::Authlib {
                    access_token: tok,
                    client_token: ctok,
                    ..
                }) = inner.data.accounts.iter_mut().find(|a| a.id() == id)
                {
                    *tok = res.access_token;
                    *ctok = res.client_token;
                }
                let view = inner
                    .data
                    .accounts
                    .iter()
                    .find(|a| a.id() == id)
                    .unwrap()
                    .to_view(is_selected);

                drop(inner);
                self.persist().await?;
                Ok(view)
            }
            StoredAccount::Offline { .. } => {
                let is_selected = {
                    let inner = self.inner.read().await;
                    inner.data.last_used.as_deref() == Some(id)
                };
                Ok(account.to_view(is_selected))
            }
            StoredAccount::YggdrasilConnect { .. } => {
                let is_selected = {
                    let inner = self.inner.read().await;
                    inner.data.last_used.as_deref() == Some(id)
                };
                Ok(account.to_view(is_selected))
            }
        }
    }

    pub async fn update_skin_head(
        &self,
        id: &str,
        data_url: String,
    ) -> Result<AccountView, AuthError> {
        let is_selected = {
            let inner = self.inner.read().await;
            inner.data.last_used.as_deref() == Some(id)
        };
        let mut inner = self.inner.write().await;
        let acc = inner
            .data
            .accounts
            .iter_mut()
            .find(|a| a.id() == id)
            .ok_or_else(|| AuthError::AccountNotFound(id.to_string()))?;

        match acc {
            StoredAccount::Microsoft {
                skin_head_data_url, ..
            }
            | StoredAccount::Offline {
                skin_head_data_url, ..
            }
            | StoredAccount::Authlib {
                skin_head_data_url, ..
            }
            | StoredAccount::YggdrasilConnect {
                skin_head_data_url, ..
            } => {
                *skin_head_data_url = Some(data_url);
            }
        }

        let view = acc.to_view(is_selected);
        drop(inner);
        self.persist().await?;
        Ok(view)
    }

    pub async fn update_cape(
        &self,
        id: &str,
        active_cape_id: Option<&str>,
    ) -> Result<AccountView, AuthError> {
        let is_selected = {
            let inner = self.inner.read().await;
            inner.data.last_used.as_deref() == Some(id)
        };
        let mut inner = self.inner.write().await;
        let acc = inner
            .data
            .accounts
            .iter_mut()
            .find(|a| a.id() == id)
            .ok_or_else(|| AuthError::AccountNotFound(id.to_string()))?;

        match acc {
            StoredAccount::Microsoft { capes, .. }
            | StoredAccount::YggdrasilConnect { capes, .. } => {
                for cape in capes.iter_mut() {
                    cape.active = Some(cape.id.as_str()) == active_cape_id;
                }
            }
            _ => {}
        }

        let view = acc.to_view(is_selected);
        drop(inner);
        self.persist().await?;
        Ok(view)
    }

    pub async fn get_account(&self, id: &str) -> Option<StoredAccount> {
        let inner = self.inner.read().await;
        inner.data.accounts.iter().find(|a| a.id() == id).cloned()
    }
}

#[async_trait]
impl AccountSessionProvider for AccountStore {
    async fn resolve_session(&self, account_id: &str) -> Result<ResolvedAccountSession, AppError> {
        let inner = self.inner.read().await;
        let account = inner
            .data
            .accounts
            .iter()
            .find(|a| a.id() == account_id)
            .ok_or_else(|| {
                AppError::new("auth.accountNotFound")
                    .with_detail(format!("account {account_id} not found"))
            })?;

        match account {
            StoredAccount::Microsoft {
                id,
                uuid,
                username,
                access_token,
                ..
            } => Ok(ResolvedAccountSession {
                id: id.clone(),
                kind: AccountKind::Microsoft,
                username: username.clone(),
                uuid: uuid.clone(),
                access_token: access_token.clone(),
                user_type: "msa".to_string(),
                api_location: None,
                xuid: None,
            }),
            StoredAccount::Offline {
                id, username, uuid, ..
            } => Ok(ResolvedAccountSession {
                id: id.clone(),
                kind: AccountKind::Offline,
                username: username.clone(),
                uuid: uuid.clone(),
                access_token: uuid.clone(),
                user_type: "legacy".to_string(),
                api_location: None,
                xuid: None,
            }),
            StoredAccount::Authlib {
                id,
                server,
                username,
                uuid,
                access_token,
                ..
            } => Ok(ResolvedAccountSession {
                id: id.clone(),
                kind: AccountKind::Authlib,
                username: username.clone(),
                uuid: uuid.clone(),
                access_token: access_token.clone(),
                user_type: "mojang".to_string(),
                api_location: Some(server.clone()),
                xuid: None,
            }),
            StoredAccount::YggdrasilConnect {
                id,
                discovery_url,
                uuid,
                username,
                access_token,
                ..
            } => Ok(ResolvedAccountSession {
                id: id.clone(),
                kind: AccountKind::YggdrasilConnect,
                username: username.clone(),
                uuid: uuid.clone(),
                access_token: access_token.clone(),
                user_type: "mojang".to_string(),
                api_location: Some(discovery_url.clone()),
                xuid: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MCLR_DATA_DIR` is process-global, so env-sensitive tests must not overlap.
    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// Builds a store in `dir` with `n` offline accounts; the last one is `last_used`.
    async fn seeded_store(dir: &std::path::Path, names: &[&str]) -> AccountStore {
        let dir_str = dir.to_str().unwrap().to_string();
        unsafe {
            std::env::set_var("MCLR_DATA_DIR", &dir_str);
        }
        let store = AccountStore::open().expect("open store");
        for name in names {
            store
                .add_offline(name, OfflineUuidMode::Standard, None)
                .await
                .expect("add offline account");
        }
        store
    }

    #[tokio::test]
    async fn test_ensure_valid_selection_keeps_valid_pointer() {
        let _guard = ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().unwrap();
        let store = seeded_store(temp.path(), &["KeepA", "KeepB"]).await;

        store.ensure_valid_selection().await.unwrap();
        let list = store.list_accounts().await;
        assert_eq!(list.len(), 2);
        assert_eq!(list.iter().filter(|a| a.selected).count(), 1);
        // KeepB was added last, so it stays selected.
        assert!(list[1].selected);
    }

    #[tokio::test]
    async fn test_ensure_valid_selection_heals_dangling_pointer_and_persists() {
        let _guard = ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().unwrap();
        let store = seeded_store(temp.path(), &["HealA", "HealB"]).await;
        let survivor_id = store.list_accounts().await[0].id.clone();

        // Simulate the selected account disappearing outside the launcher.
        {
            let mut inner = store.inner.write().await;
            inner.data.last_used = Some("ghost-account-id".to_string());
            inner.data.accounts.truncate(1);
        }

        // Nothing is selected until the repair runs.
        let before = store.list_accounts().await;
        assert_eq!(before.len(), 1);
        assert!(!before[0].selected);

        store.ensure_valid_selection().await.unwrap();
        let healed = store.list_accounts().await;
        assert_eq!(healed.len(), 1);
        assert_eq!(healed[0].id, survivor_id);
        assert!(healed[0].selected);

        // A second repair pass is a no-op and must not disturb the healed pointer.
        store.ensure_valid_selection().await.unwrap();
        let again = store.list_accounts().await;
        assert_eq!(again[0].id, survivor_id);
        assert!(again[0].selected);
    }

    #[tokio::test]
    async fn test_ensure_valid_selection_with_empty_store_clears_pointer() {
        let _guard = ENV_LOCK.lock().await;
        let temp = tempfile::tempdir().unwrap();
        let store = seeded_store(temp.path(), &[]).await;
        {
            let mut inner = store.inner.write().await;
            inner.data.last_used = Some("ghost-account-id".to_string());
        }

        store.ensure_valid_selection().await.unwrap();
        let inner = store.inner.read().await;
        assert!(inner.data.last_used.is_none());
    }
}
