#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn check_and_apply_gpu_setting() {
    let config_path = mclr_core::paths::config_file();
    if config_path.exists()
        && let Ok(content) = std::fs::read_to_string(&config_path)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
        && val
            .get("system")
            .and_then(|s| s.get("disableGpu"))
            .and_then(|d| d.as_bool())
            .unwrap_or(false)
    {
        unsafe {
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--disable-gpu");
        }
    }
}

fn main() {
    check_and_apply_gpu_setting();
    mclr_app_lib::run();
}
