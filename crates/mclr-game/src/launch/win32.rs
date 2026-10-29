//! Windows API integration for process window discovery and title polling.

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, OpenProcess, PROCESS_SET_INFORMATION,
    SetPriorityClass,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, SetWindowTextW,
};
use windows::core::{BOOL, PCWSTR};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

/// Registers `java_exe` as a high-performance GPU app in Windows graphics settings.
///
/// Writes `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` with
/// `GpuPreference=2;`, the same value the Windows GUI produces. Failures are
/// logged and ignored: the launch must never be blocked by a preference write.
pub fn set_gpu_high_performance(java_exe: &Path) {
    const KEY: &str = r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences";
    let status = Command::new("reg")
        .args([
            "add",
            KEY,
            "/v",
            &java_exe.to_string_lossy(),
            "/t",
            "REG_SZ",
            "/d",
            "GpuPreference=2;",
            "/f",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => tracing::warn!("GPU preference registry write exited with {s}"),
        Err(e) => tracing::warn!("Failed to write GPU preference registry value: {e}"),
    }
}

/// Applies the configured process priority to a freshly spawned game process.
pub fn apply_process_priority(pid: u32, priority: &str) {
    let class = match priority {
        "aboveNormal" | "above-normal" => ABOVE_NORMAL_PRIORITY_CLASS,
        "high" => HIGH_PRIORITY_CLASS,
        _ => return,
    };
    let handle = match unsafe { OpenProcess(PROCESS_SET_INFORMATION, false, pid) } {
        Ok(h) if !h.is_invalid() => h,
        Ok(_) => return,
        Err(e) => {
            tracing::warn!("OpenProcess for priority failed: {e}");
            return;
        }
    };
    if let Err(e) = unsafe { SetPriorityClass(handle, class) } {
        tracing::warn!("SetPriorityClass failed: {e}");
    }
    let _ = unsafe { CloseHandle(handle) };
}
