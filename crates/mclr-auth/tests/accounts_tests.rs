use mclr_auth::microsoft::{MicrosoftAuthService, MsAuthEndpoints};
use mclr_auth::offline::{OfflineUuidMode, compute_offline_uuid};
use mclr_auth::skin::crop_skin_head;
use mclr_auth::store::AccountStore;
use mclr_core::AppError;
use reqwest::Client;
use std::io::Cursor;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_offline_uuid_steve_rfc4122_v3() {
    let uuid = compute_offline_uuid("Steve", OfflineUuidMode::Standard, None).unwrap();
    assert_eq!(uuid, "5627dd98e6be3c21b8a8e92344183641");
}

#[tokio::test]
async fn test_store_dpapi_aes_gcm_roundtrip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir_str = temp_dir.path().to_str().unwrap().to_string();

    unsafe {
        std::env::set_var("MCLR_DATA_DIR", &data_dir_str);
    }

    let store = AccountStore::open().expect("failed to open store");
    let acc1 = store
        .add_offline("Alex", OfflineUuidMode::Standard, None, None)
        .await
        .expect("failed to add offline account");

    assert_eq!(acc1.username, "Alex");
    assert!(acc1.selected);

    let accounts = store.list_accounts().await;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, acc1.id);

    // Verify files on disk
    let enc_file = temp_dir.path().join("accounts.enc");
    let key_file = temp_dir.path().join("master.key");
    assert!(enc_file.exists());
    assert!(key_file.exists());

    let raw_enc = std::fs::read(&enc_file).unwrap();
    assert_eq!(&raw_enc[0..4], b"MCLR");
    assert_eq!(raw_enc[4], 1u8);

    // Reopen store from disk
    let store2 = AccountStore::open().expect("failed to reopen store");
    let accounts2 = store2.list_accounts().await;
    assert_eq!(accounts2.len(), 1);
    assert_eq!(accounts2[0].id, acc1.id);
    assert_eq!(accounts2[0].username, "Alex");

    unsafe {
        std::env::remove_var("MCLR_DATA_DIR");
    }
}

#[tokio::test]
async fn test_microsoft_flow_wiremock() {
    let server = MockServer::start().await;

    // Step 1: Device code
    Mock::given(method("POST"))
        .and(path("/devicecode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dev-12345",
            "user_code": "ABC-DEF",
            "verification_uri": "https://microsoft.com/link",
            "expires_in": 900,
            "interval": 1
        })))
        .mount(&server)
        .await;

    // Step 2: Token endpoint with authorization_pending -> slow_down -> success
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({ "error": "authorization_pending" })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(serde_json::json!({ "error": "slow_down" })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ms-access-token-123",
            "refresh_token": "ms-refresh-token-456",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;

    // Step 3: Xbox Live
    Mock::given(method("POST"))
        .and(path("/xbl/authenticate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "IssueInstant": "2026-10-24T00:00:00Z",
            "NotAfter": "2026-10-25T00:00:00Z",
            "Token": "xbl-token-xyz"
        })))
        .mount(&server)
        .await;

    // Step 4: XSTS
    Mock::given(method("POST"))
        .and(path("/xsts/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "IssueInstant": "2026-10-24T00:00:00Z",
            "NotAfter": "2026-10-25T00:00:00Z",
            "Token": "xsts-token-xyz",
            "DisplayClaims": {
                "xui": [{ "uhs": "user-hash-789" }]
            }
        })))
        .mount(&server)
        .await;

    // Step 5: Minecraft login
    Mock::given(method("POST"))
        .and(path("/mc/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mc-access-token-999",
            "expires_in": 86400
        })))
        .mount(&server)
        .await;

    // Step 6: Store entitlements & Profile
    Mock::given(method("GET"))
        .and(path("/mc/store"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [{ "name": "product_minecraft" }]
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/mc/profile"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "5627dd98e6be3c21b8a8e92344183641",
            "name": "Steve",
            "skins": [],
            "capes": []
        })))
        .mount(&server)
        .await;

    let endpoints = MsAuthEndpoints {
        device_code: format!("{}/devicecode", server.uri()),
        token: format!("{}/token", server.uri()),
        xbox_live: format!("{}/xbl/authenticate", server.uri()),
        xsts: format!("{}/xsts/authorize", server.uri()),
        minecraft_login: format!("{}/mc/login", server.uri()),
        mc_store: format!("{}/mc/store", server.uri()),
        mc_profile: format!("{}/mc/profile", server.uri()),
    };

    let client = Client::new();
    let service = MicrosoftAuthService::new(client, endpoints, Some("test-client-id".to_string()))
        .expect("service creation");

    let device = service.get_device_code().await.expect("device code");
    assert_eq!(device.user_code, "ABC-DEF");

    let (ms_tok, refresh_tok) = service
        .poll_token(&device.device_code, 1)
        .await
        .expect("poll token");
    assert_eq!(ms_tok, "ms-access-token-123");
    assert_eq!(refresh_tok, "ms-refresh-token-456");

    let result = service
        .complete_flow(&ms_tok, &refresh_tok)
        .await
        .expect("complete flow");
    assert_eq!(result.username, "Steve");
    assert_eq!(result.uuid, "5627dd98e6be3c21b8a8e92344183641");
    assert_eq!(result.access_token, "mc-access-token-999");
}

#[tokio::test]
async fn test_xsts_xerr_error_mappings() {
    let test_cases: [(i64, &str); 4] = [
        (2148916227_i64, "auth.xsts.banned"),
        (2148916233_i64, "auth.xsts.noXboxAccount"),
        (2148916235_i64, "auth.xsts.regionBlocked"),
        (2148916238_i64, "auth.xsts.minor"),
    ];

    for (xerr_code, expected_code) in test_cases {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/xsts/authorize"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "XErr": xerr_code,
                "Message": "Error",
                "Redirect": "https://xbox.com"
            })))
            .mount(&server)
            .await;

        let endpoints = MsAuthEndpoints {
            device_code: String::new(),
            token: String::new(),
            xbox_live: String::new(),
            xsts: format!("{}/xsts/authorize", server.uri()),
            minecraft_login: String::new(),
            mc_store: String::new(),
            mc_profile: String::new(),
        };

        let client = Client::new();
        let service =
            MicrosoftAuthService::new(client, endpoints, Some("test-client-id".into())).unwrap();

        let err = service.authorize_xsts("mock-xbl").await.unwrap_err();
        assert_eq!(err.code(), expected_code);

        let app_err = AppError::from(err);
        assert_eq!(app_err.code, expected_code);
    }
}

#[tokio::test]
async fn test_missing_client_id_returns_not_configured() {
    unsafe {
        std::env::remove_var("MCLR_MS_CLIENT_ID");
    }

    let client = Client::new();
    let err = MicrosoftAuthService::new(client, MsAuthEndpoints::default(), None).unwrap_err();
    assert_eq!(err.code(), "notConfigured");

    let app_err = AppError::from(err);
    assert_eq!(app_err.code, "notConfigured");
    assert_eq!(
        app_err.params.get("what").map(|s| s.as_str()),
        Some("microsoft")
    );
}

#[test]
fn test_skin_head_cropping_rgba() {
    use image::{ImageFormat, Rgba, RgbaImage};

    let mut skin = RgbaImage::new(64, 64);
    for y in 8..16 {
        for x in 8..16 {
            skin.put_pixel(x, y, Rgba([100, 150, 200, 255]));
        }
    }
    for y in 8..16 {
        for x in 40..48 {
            skin.put_pixel(x, y, Rgba([255, 0, 0, 128]));
        }
    }

    let mut buf = Cursor::new(Vec::new());
    skin.write_to(&mut buf, ImageFormat::Png).unwrap();
    let png_bytes = buf.into_inner();

    let data_url = crop_skin_head(&png_bytes, "test_crop_account").unwrap();
    assert!(data_url.starts_with("data:image/png;base64,"));
}
