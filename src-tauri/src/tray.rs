//! System tray: the launcher can keep running with its window hidden.
//!
//! The tray is always present so the window can be hidden/shown at any time; the
//! `system.keep_in_background` setting only decides what the window close button
//! does (hide to tray vs. quit). The close handler runs on the window thread and
//! must not await the settings store, so the flag lives in an atomic that
//! `settings_patch` keeps in sync.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    AppHandle, Manager, Runtime,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// Shared, lock-free mirror of `system.keep_in_background`.
#[derive(Default)]
pub struct BackgroundResidency {
    keep: AtomicBool,
}

impl BackgroundResidency {
    pub fn set(&self, keep: bool) {
        self.keep.store(keep, Ordering::Relaxed);
    }

    pub fn keep(&self) -> bool {
        self.keep.load(Ordering::Relaxed)
    }
}

/// Bring the main window back from the tray (or from a minimized/occluded state).
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn hide_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Build the tray icon and its menu. Must run inside the app's `setup` hook.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "隐藏到托盘", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 MCLR", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &hide, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("MCLR")
        .menu(&menu)
        // Left click restores the window; the menu stays on right click.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "hide" => hide_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}
