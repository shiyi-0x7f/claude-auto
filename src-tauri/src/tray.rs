//! 系统托盘与窗口显隐。

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::service::Service;
use crate::state::Snapshot;

pub const MAIN: &str = "main";
pub const WIDGET: &str = "widget";
const TRAY_ID: &str = "tray";

pub fn show_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn toggle_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
        }
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开设置", true, None::<&str>)?;
    let widget = MenuItem::with_id(app, "widget", "显示/隐藏挂件", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新额度", true, None::<&str>)?;
    let apply = MenuItem::with_id(app, "apply", "执行推荐切换", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &widget, &refresh, &apply, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Claude Auto")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let svc = app.state::<Arc<Service>>().inner().clone();
            match event.id.as_ref() {
                "open" => show_window(app, MAIN),
                "widget" => toggle_widget(app),
                "refresh" => svc.wake.notify_one(),
                "apply" => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = svc.apply_pending().await {
                            svc.notify("无法切换", &e.to_string());
                        }
                    });
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle(), WIDGET);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn update_tooltip(app: &AppHandle, snap: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let text = match snap.accounts.iter().find(|a| a.is_active) {
        Some(a) => {
            let pct = |w: Option<f64>| w.map_or("--".to_string(), |v| format!("{v:.0}%"));
            let u = a.usage.as_ref();
            let label = if snap.settings.privacy_mode {
                crate::privacy::mask(&a.label)
            } else {
                a.label.clone()
            };
            format!(
                "Claude Auto · {}\n5h {} · 周 {}",
                label,
                pct(u.and_then(|u| u.five_hour).map(|w| w.utilization)),
                pct(u.and_then(|u| u.seven_day).map(|w| w.utilization)),
            )
        }
        None => "Claude Auto · 当前账号未纳管".into(),
    };
    let _ = tray.set_tooltip(Some(text));
}
