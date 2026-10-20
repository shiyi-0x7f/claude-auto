import type { AccountView, Settings, UsageWindow } from "../api/types";

/** 已过重置时间的窗口视为已恢复（与后端 policy::effective_util 一致） */
export function effectiveUtil(w: UsageWindow | null | undefined, now: number): number | null {
  if (!w) return null;
  if (w.resetsAt !== null && w.resetsAt <= now) return 0;
  return w.utilization;
}

export function formatPct(v: number | null): string {
  return v === null ? "--" : `${Math.round(v)}%`;
}

/** 距重置的剩余时间：「3天4时」「2时13分」「8分」「即将重置」 */
export function formatCountdown(resetsAt: number | null | undefined, now: number): string {
  if (!resetsAt) return "--";
  const ms = resetsAt - now;
  if (ms <= 0) return "即将重置";
  const mins = Math.floor(ms / 60_000);
  const days = Math.floor(mins / 1440);
  const hours = Math.floor((mins % 1440) / 60);
  const m = mins % 60;
  if (days > 0) return `${days}天${hours}时`;
  if (hours > 0) return `${hours}时${m}分`;
  return `${Math.max(m, 1)}分`;
}

/** HUD 用的紧凑倒计时：「3d04h」「2h13m」「08m」「00m」 */
export function formatCountdownCompact(resetsAt: number | null | undefined, now: number): string {
  if (!resetsAt) return "--";
  const mins = Math.max(Math.floor((resetsAt - now) / 60_000), 0);
  const pad = (n: number) => String(n).padStart(2, "0");
  const days = Math.floor(mins / 1440);
  const hours = Math.floor((mins % 1440) / 60);
  if (days > 0) return `${days}d${pad(hours)}h`;
  if (hours > 0) return `${hours}h${pad(mins % 60)}m`;
  return `${pad(mins)}m`;
}

const DAY_MS = 86_400_000;

/** 剩余整天数（向下取整，已过期为负），与后端 expiry::days_left 一致 */
export function daysLeft(at: number, now: number): number {
  return Math.floor((at - now) / DAY_MS);
}

/** 登录有效期提醒阈值（天），与后端 RELOGIN_WARN_DAYS 一致 */
export const RELOGIN_WARN_DAYS = 7;

/** 「11-17」 */
export function formatDate(ts: number): string {
  const d = new Date(ts);
  return `${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

/** 挂件隐私模式：只保留账号名前两个字 */
export function shortLabel(label: string): string {
  return [...label].slice(0, 2).join("");
}

export function formatTime(ts: number | null): string {
  if (!ts) return "--";
  const d = new Date(ts);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export type Level = "ok" | "warn" | "danger" | "unknown";

/** 单个窗口的紧张程度：≥阈值 danger；≥阈值-20 warn */
export function levelOf(util: number | null, threshold: number): Level {
  if (util === null) return "unknown";
  if (util >= threshold) return "danger";
  if (util >= threshold - 20) return "warn";
  return "ok";
}

export type Mood = "happy" | "worried" | "sleepy" | "confused";

/** 宠物心情：取两个窗口里更紧张的那个 */
export function moodOf(acc: AccountView | undefined, s: Settings, now: number): Mood {
  if (!acc?.usage) return "confused";
  const levels = [
    levelOf(effectiveUtil(acc.usage.fiveHour, now), s.fiveHourThreshold),
    levelOf(effectiveUtil(acc.usage.sevenDay, now), s.sevenDayThreshold),
  ];
  if (levels.includes("danger")) return "sleepy";
  if (levels.includes("warn")) return "worried";
  if (levels.every((l) => l === "unknown")) return "confused";
  return "happy";
}

const PLAN_LABELS: Record<string, string> = {
  pro: "Pro",
  max: "Max",
  team: "Team",
  enterprise: "Enterprise",
  free: "Free",
};

export function planLabel(plan: string | null): string {
  if (!plan) return "未知";
  return PLAN_LABELS[plan] ?? plan;
}
