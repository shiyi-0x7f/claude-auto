import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { ApiError, Settings, Snapshot } from "./types";

export const SNAPSHOT_EVENT = "snapshot";

export const api = {
  getSnapshot: () => invoke<Snapshot>("get_snapshot"),
  refreshNow: () => invoke<void>("refresh_now"),
  switchAccount: (id: string) => invoke<void>("switch_account", { id }),
  applyPending: () => invoke<void>("apply_pending"),
  dismissPending: () => invoke<void>("dismiss_pending"),
  importCurrent: () => invoke<string>("import_current"),
  startLogin: () => invoke<string>("start_login"),
  completeLogin: (code: string) => invoke<string>("complete_login", { code }),
  cancelLogin: () => invoke<void>("cancel_login"),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  renameAccount: (id: string, label: string) => invoke<void>("rename_account", { id, label }),
  /** at 为 null 时恢复按月推算 */
  setSubscriptionExpiry: (id: string, at: number | null) =>
    invoke<void>("set_subscription_expiry", { id, at }),
  updateSettings: (settings: Settings) => invoke<void>("update_settings", { settings }),
  openMain: () => invoke<void>("open_main"),
};

export function onSnapshot(cb: (s: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>(SNAPSHOT_EVENT, (e) => cb(e.payload));
}

export function toApiError(e: unknown): ApiError {
  if (typeof e === "object" && e !== null && "message" in e) {
    const err = e as { code?: unknown; message: unknown };
    return { code: String(err.code ?? "UNKNOWN"), message: String(err.message) };
  }
  return { code: "UNKNOWN", message: String(e) };
}
