//! Windows API integration for process window discovery and title polling.

use std::time::Duration;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, SetWindowTextW,
};
use windows::core::PCWSTR;

/// Finds the main window handle (HWND) belonging to the given process ID.
pub fn find_window_for_pid(pid: u32) -> Option<HWND> {
    struct SearchContext {
        target_pid: u32,
        found_hwnd: Option<HWND>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let ctx = &mut *(lparam.0 as *mut SearchContext);
            let mut window_pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
            if window_pid == ctx.target_pid {
                let length = GetWindowTextLengthW(hwnd);
                if length > 0 {
                    let mut buf = vec![0u16; (length + 1) as usize];
                    let copied = GetWindowTextW(hwnd, &mut buf);
                    if copied > 0 {
                        let title = String::from_utf16_lossy(&buf[..copied as usize]);
                        if !title.starts_with("GLFW") && title != "PopupMessageWindow" {
                            ctx.found_hwnd = Some(hwnd);
                            return BOOL(0);
                        }
                    }
                }
            }
            BOOL(1)
        }
    }

    let mut ctx = SearchContext {
        target_pid: pid,
        found_hwnd: None,
    };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.found_hwnd
}

/// Sets the window text for the top-level window of the given PID.
pub fn set_window_title_for_pid(pid: u32, title: &str) -> bool {
    if let Some(hwnd) = find_window_for_pid(pid) {
        let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { SetWindowTextW(hwnd, PCWSTR::from_raw(wide.as_ptr())).is_ok() }
    } else {
        false
    }
}

/// Spawns background task to set Minecraft window title when ready.
pub(crate) fn spawn_window_title_poller(pid: u32, template: String) {
    if template.trim().is_empty() {
        return;
    }

    tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        while tokio::time::Instant::now() < deadline {
            let now = chrono::Local::now();
            let expanded = template
                .replace("{date}", &now.format("%Y-%m-%d").to_string())
                .replace("{time}", &now.format("%H:%M:%S").to_string());

            let success =
                tokio::task::spawn_blocking(move || set_window_title_for_pid(pid, &expanded))
                    .await
                    .unwrap_or(false);

            if success {
                // Keep polling briefly to ensure title sticks
                for _ in 0..5 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    let now2 = chrono::Local::now();
                    let expanded2 = template
                        .replace("{date}", &now2.format("%Y-%m-%d").to_string())
                        .replace("{time}", &now2.format("%H:%M:%S").to_string());
                    let _ = tokio::task::spawn_blocking(move || {
                        set_window_title_for_pid(pid, &expanded2)
                    })
                    .await;
                }
                break;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}
