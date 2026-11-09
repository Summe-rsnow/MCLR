//! Account management IPC commands.

pub use mclr_auth::{
    AccountStore, AccountView, AuthlibLoginResponse, AuthlibProfileView, CapeView, DeviceCodeEvent,
    OfflineUuidMode,
};
use mclr_auth::{
    AuthError, MicrosoftAuthService, MsAuthEndpoints, StoredAccount, crop_skin_head,
    default_skin_data_url, png_data_url, set_active_cape, upload_authlib_skin, upload_skin,
};
use mclr_core::{AppError, TaskCenter};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
#[specta::specta]
pub async fn accounts_list(
    state: State<'_, Arc<AccountStore>>,
) -> Result<Vec<AccountView>, AppError> {
    state.list_accounts_healed().await.map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_select(
    state: State<'_, Arc<AccountStore>>,
    id: String,
) -> Result<Vec<AccountView>, AppError> {
    state.select_account(&id).await.map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_remove(
    state: State<'_, Arc<AccountStore>>,
    id: String,
) -> Result<Vec<AccountView>, AppError> {
    state.remove_account(&id).await.map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_add_offline(
    state: State<'_, Arc<AccountStore>>,
    username: String,
    uuid_mode: String,
    custom_uuid: Option<String>,
    skin_model: Option<String>,
) -> Result<AccountView, AppError> {
    let mode: OfflineUuidMode = uuid_mode.parse().map_err(AppError::from)?;
    state
        .add_offline(
            &username,
            mode,
            custom_uuid.as_deref(),
            skin_model.as_deref(),
        )
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_rename_offline(
    state: State<'_, Arc<AccountStore>>,
    id: String,
    username: String,
) -> Result<AccountView, AppError> {
    state
        .rename_offline(&id, &username)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_add_microsoft(
    http: State<'_, crate::http::HttpState>,
    app: AppHandle,
    state: State<'_, Arc<AccountStore>>,
    _tasks_state: State<'_, Arc<TaskCenter>>,
) -> Result<String, AppError> {
    let client = http.client();
    let service = MicrosoftAuthService::new(client, MsAuthEndpoints::default(), None)
        .map_err(AppError::from)?;

    let mut params = HashMap::new();
    params.insert("type".to_string(), "microsoft".to_string());

    let plan = mclr_core::tasks::TaskPlan {
        title_key: "task.auth.microsoft".into(),
        title_params: params,
        steps: vec![
            mclr_core::tasks::StepPlan {
                key: "deviceCode",
                weight: 10.0,
            },
            mclr_core::tasks::StepPlan {
                key: "authorize",
                weight: 60.0,
            },
            mclr_core::tasks::StepPlan {
                key: "profile",
                weight: 30.0,
            },
        ],
    };

    let engine = app.state::<Arc<mclr_core::tasks::TaskEngine>>();
    let ctx = engine.begin_tracked("auth.microsoft", plan).await;
    let task_id = ctx.id.clone();
    let store = Arc::clone(&state);
    let app_handle = app.clone();

    tokio::spawn(async move {
        ctx.begin_step("deviceCode").await;

        let device = match service.get_device_code().await {
            Ok(d) => d,
            Err(e) => {
                ctx.finish(Err(e.into())).await;
                return;
            }
        };
        ctx.finish_step("deviceCode").await;

        let _ = app_handle.emit(
            "accounts://deviceCode",
            DeviceCodeEvent {
                user_code: device.user_code.clone(),
                verification_uri: device.verification_uri.clone(),
                expires_in: device.expires_in,
            },
        );

        ctx.begin_step("authorize").await;
        let (ms_access_token, refresh_token) = match service
            .poll_token(&device.device_code, device.interval, Some(ctx.control()))
            .await
        {
            Ok(tokens) => tokens,
            Err(e) => {
                ctx.finish(Err(e.into())).await;
                return;
            }
        };
        ctx.finish_step("authorize").await;

        ctx.begin_step("profile").await;
        let login_result = match service
            .complete_flow(&ms_access_token, &refresh_token)
            .await
        {
            Ok(res) => res,
            Err(e) => {
                ctx.finish(Err(e.into())).await;
                return;
            }
        };

        match store.add_microsoft(login_result).await {
            Ok(_) => {
                ctx.finish_step("profile").await;
                ctx.finish(Ok(())).await;
            }
            Err(e) => {
                ctx.finish(Err(e.into())).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_add_authlib(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, Arc<AccountStore>>,
    server: String,
    username: String,
    password: String,
) -> Result<AuthlibLoginResponse, AppError> {
    let client = http.client();
    state
        .add_authlib(&client, &server, &username, &password)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_select_profile(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, Arc<AccountStore>>,
    pending_session_id: String,
    profile_id: String,
) -> Result<AccountView, AppError> {
    let client = http.client();
    state
        .select_authlib_profile(&client, &pending_session_id, &profile_id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_refresh(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, Arc<AccountStore>>,
    id: String,
) -> Result<AccountView, AppError> {
    let client = http.client();
    state
        .refresh_account(&client, &id)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_get_skin_data_url(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, Arc<AccountStore>>,
    id: String,
) -> Result<String, AppError> {
    let account = state
        .get_account(&id)
        .await
        .ok_or_else(|| AppError::new("auth.accountNotFound"))?;

    let client = http.client();
    match &account {
        StoredAccount::Offline { uuid, .. } => Ok(default_skin_data_url(uuid)),
        StoredAccount::Microsoft { uuid, .. } => {
            let profile_url =
                format!("https://sessionserver.mojang.com/session/minecraft/profile/{uuid}");
            match mclr_auth::yggdrasil::fetch_skin_png_from_profile_url(&client, &profile_url).await
            {
                Ok(bytes) => Ok(png_data_url(&bytes)),
                // Accounts without a custom skin still export the bundled default texture.
                Err(AuthError::SkinNotFound) => Ok(default_skin_data_url(uuid)),
                Err(e) => Err(AppError::from(e)),
            }
        }
        StoredAccount::Authlib { server, uuid, .. } => {
            let api_root = mclr_auth::yggdrasil::normalize_server_url(server);
            match mclr_auth::yggdrasil::fetch_profile_skin_png(&client, &api_root, uuid).await {
                Ok(bytes) => Ok(png_data_url(&bytes)),
                Err(AuthError::SkinNotFound) => Ok(default_skin_data_url(uuid)),
                Err(e) => Err(AppError::from(e)),
            }
        }
        StoredAccount::YggdrasilConnect { uuid, .. } => Ok(default_skin_data_url(uuid)),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_skin_upload(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, Arc<AccountStore>>,
    id: String,
    variant: String,
    png_path: String,
) -> Result<AccountView, AppError> {
    let client = http.client();
    let account = state
        .get_account(&id)
        .await
        .ok_or_else(|| AppError::new("auth.accountNotFound"))?;

    let path = Path::new(&png_path);
    match &account {
        StoredAccount::Microsoft { access_token, .. } => {
            upload_skin(&client, access_token, &variant, path)
                .await
                .map_err(AppError::from)?;
        }
        StoredAccount::Authlib {
            server,
            server_name,
            uuid,
            access_token,
            ..
        } => {
            // Authlib servers accept the same skin file through their own endpoint;
            // silently skipping it used to look like a successful upload.
            let api_root = mclr_auth::yggdrasil::normalize_server_url(server);
            upload_authlib_skin(&client, &api_root, access_token, uuid, &variant, path)
                .await
                .map_err(|e| {
                    let label = server_name.as_deref().unwrap_or(server.as_str());
                    AppError::new("auth.skinUploadFailed")
                        .with_param("server", label.to_string())
                        .with_detail(e.to_string())
                })?;
        }
        _ => {
            return Err(AppError::new("auth.skinUploadUnsupported"));
        }
    }

    let id_for_crop = id.clone();
    let path_buf = path.to_path_buf();
    let data_url = super::run_blocking(move || {
        let bytes = std::fs::read(&path_buf).map_err(mclr_auth::AuthError::Io)?;
        crop_skin_head(&bytes, &id_for_crop)
    })
    .await?;
    state
        .update_skin_head(&id, data_url)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn accounts_cape_set(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, Arc<AccountStore>>,
    id: String,
    cape_id: Option<String>,
) -> Result<AccountView, AppError> {
    let client = http.client();
    let account = state
        .get_account(&id)
        .await
        .ok_or_else(|| AppError::new("auth.accountNotFound"))?;

    if let StoredAccount::Microsoft { access_token, .. } = &account {
        set_active_cape(&client, access_token, cape_id.as_deref())
            .await
            .map_err(AppError::from)?;
    }

    state
        .update_cape(&id, cape_id.as_deref())
        .await
        .map_err(AppError::from)
}
