mod anthropic;
mod cli_files;
mod commands;
mod error;
mod fsutil;
mod login;
mod policy;
mod privacy;
mod proxy;
mod service;
mod settings;
mod state;
mod tray;
mod vault;

use std::sync::Arc;

use tauri::{Manager, PhysicalPosition, WindowEvent};

use crate::cli_files::CliPaths;
use crate::service::Service;
use crate::settings::SettingsStore;
use crate::state::Inner;
use crate::vault::Vault;

fn init_logging(data_dir: &std::path::Path) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "claude_auto_lib=info".into());
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if cfg!(debug_assertions) {
        builder.init();
        return;
    }
    let _ = std::fs::create_dir_all(data_dir);
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir.join("claude-auto.log"))
    {
        Ok(f) => builder
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(f))
            .init(),
        Err(_) => builder.init(),
    }
}

/// 挂件默认停靠在主屏右下角（留出任务栏）。
fn place_widget(app: &tauri::App) {
    let Some(w) = app.get_webview_window(tray::WIDGET) else {
        return;
    };
    if let (Ok(Some(mon)), Ok(size)) = (w.primary_monitor(), w.outer_size()) {
        let area = mon.size();
        let margin = (24.0 * mon.scale_factor()) as i32;
        let bottom = (72.0 * mon.scale_factor()) as i32;
        let x = area.width as i32 - size.width as i32 - margin;
        let y = area.height as i32 - size.height as i32 - bottom;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    init_logging(&data_dir);

    let paths = CliPaths::resolve(&app.path().home_dir()?);
    tracing::info!(config_dir = %paths.config_dir.display(), "CLI 配置目录");
    let inner = Inner::new(Vault::load(&data_dir)?, SettingsStore::load(&data_dir)?);
    let svc = Arc::new(Service::new(app.handle().clone(), inner, paths)?);
    app.manage(Arc::clone(&svc));

    tray::setup(app.handle())?;
    place_widget(app);

    tauri::async_runtime::spawn(async move {
        loop {
            svc.poll().await;
            let wait = svc.poll_interval();
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = svc.wake.notified() => {}
            }
        }
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_window(app, tray::MAIN);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(setup)
        .on_window_event(|window, event| {
            // 关闭窗口只隐藏：监控在后台持续运行，退出走托盘菜单
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh_now,
            commands::switch_account,
            commands::apply_pending,
            commands::dismiss_pending,
            commands::import_current,
            commands::start_login,
            commands::complete_login,
            commands::cancel_login,
            commands::remove_account,
            commands::rename_account,
            commands::update_settings,
            commands::open_main,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
