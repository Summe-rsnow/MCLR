//! Account management (Offline, Microsoft, Authlib-Injector, Yggdrasil Connect) and skin/cape management.

pub mod dpapi;
pub mod error;
pub mod microsoft;
pub mod offline;
pub mod skin;
pub mod store;
pub mod yggdrasil;

pub use error::AuthError;
pub use microsoft::{DeviceCodeEvent, MicrosoftAuthService, MicrosoftLoginResult, MsAuthEndpoints};
pub use offline::{OfflineUuidMode, compute_offline_uuid, default_skin_model};
pub use skin::{
    CapeView, crop_head_png, crop_skin_head, default_head_data_url, generate_head_avatar,
    set_active_cape, upload_authlib_skin, upload_skin,
};
pub use store::{AccountStore, AccountView, AuthlibLoginResponse, StoredAccount};
pub use yggdrasil::{AuthlibProfileView, fetch_profile_head};
