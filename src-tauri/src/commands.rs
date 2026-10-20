//! 前端可调用的命令（薄层，业务在 service）。

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::service::Service;
use crate::settings::Settings;
use crate::state::Snapshot;

type Svc<'a> = State<'a, Arc<Service>>;

#[tauri::command]
pub fn get_snapshot(svc: Svc<'_>) -> Snapshot {
    svc.snapshot()
}

#[tauri::command]
pub fn refresh_now(svc: Svc<'_>) {
    svc.wake.notify_one();
}

#[tauri::command]
pub async fn switch_account(svc: Svc<'_>, id: String) -> AppResult<()> {
    svc.switch_to(&id).await
}

#[tauri::command]
pub async fn apply_pending(svc: Svc<'_>) -> AppResult<()> {
    svc.apply_pending().await
}

#[tauri::command]
pub fn dismiss_pending(svc: Svc<'_>) {
    svc.dismiss_pending();
}

#[tauri::command]
pub async fn import_current(svc: Svc<'_>) -> AppResult<String> {
    svc.import_current().await
}

/// 返回授权链接，前端展示供用户复制/打开
#[tauri::command]
pub fn start_login(svc: Svc<'_>) -> String {
    svc.login_start()
}

#[tauri::command]
pub async fn complete_login(svc: Svc<'_>, code: String) -> AppResult<String> {
    svc.login_complete(&code).await
}

#[tauri::command]
pub fn cancel_login(svc: Svc<'_>) {
    svc.login_cancel();
}

#[tauri::command]
pub async fn remove_account(svc: Svc<'_>, id: String) -> AppResult<()> {
    svc.remove(&id).await
}

#[tauri::command]
pub fn rename_account(svc: Svc<'_>, id: String, label: String) -> AppResult<()> {
    svc.rename(&id, &label)
}

#[tauri::command]
pub fn update_settings(svc: Svc<'_>, settings: Settings) -> AppResult<()> {
    svc.update_settings(settings)
}

#[tauri::command]
pub fn open_main(app: AppHandle) {
    crate::tray::show_window(&app, crate::tray::MAIN);
}
