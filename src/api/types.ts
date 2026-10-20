// 与 src-tauri/src/state.rs、settings.rs、anthropic.rs 的 DTO 一一对应（serde camelCase）

export interface UsageWindow {
  /** 已用百分比 0~100 */
  utilization: number;
  /** 窗口重置时间（毫秒时间戳） */
  resetsAt: number | null;
}

export interface Usage {
  fiveHour: UsageWindow | null;
  sevenDay: UsageWindow | null;
}

export interface AccountView {
  id: string;
  label: string;
  email: string | null;
  plan: string | null;
  isActive: boolean;
  tokenExpiresAt: number | null;
  usage: Usage | null;
  error: string | null;
  needsRelogin: boolean;
  checkedAt: number | null;
}

export type SwitchMode = "semi" | "auto";

/** cli：读取 CLI settings.json 的 env 代理；custom：使用 proxyUrl；none：直连 */
export type ProxyMode = "cli" | "custom" | "none";

export interface Settings {
  mode: SwitchMode;
  fiveHourThreshold: number;
  sevenDayThreshold: number;
  pollSecs: number;
  proxyMode: ProxyMode;
  proxyUrl: string;
  /** 隐私模式：遮挡账号名（防录屏） */
  privacyMode: boolean;
}

export type Exhaustion = "fiveHour" | "sevenDay";

export interface PendingView {
  fromId: string;
  fromLabel: string;
  toId: string | null;
  toLabel: string | null;
  reason: Exhaustion;
  reasonLabel: string;
}

export interface SwitchRecord {
  at: number;
  fromLabel: string | null;
  toLabel: string;
  trigger: string;
}

export interface Snapshot {
  accounts: AccountView[];
  activeId: string | null;
  cliLoggedIn: boolean;
  cliConfigDir: string;
  settings: Settings;
  pending: PendingView | null;
  history: SwitchRecord[];
  /** 进行中的登录授权链接 */
  loginUrl: string | null;
  /** 当前生效代理（已隐去凭据），null 为直连 */
  proxy: string | null;
  lastError: string | null;
  lastPollAt: number | null;
}

/** 后端 AppError 序列化形态 */
export interface ApiError {
  code: string;
  message: string;
}
