import type { Level, Mood } from "./utils/format";

/** 设计令牌：赛博霓虹（青 → 紫），挂件与主界面共用 */
export const COLORS = {
  brand: "#22D3EE",
  claude: "#D97757",
  brandAlt: "#A78BFA",
  ok: "#34F5C5",
  warn: "#FFB020",
  danger: "#FF3D71",
  unknown: "#6B7280",
  ink: "#05070D",
} as const;

export const LEVEL_COLOR: Record<Level, string> = {
  ok: COLORS.ok,
  warn: COLORS.warn,
  danger: COLORS.danger,
  unknown: COLORS.unknown,
};

/** 挂件随心情变化的主发光色 / 辅助色（开心时为 Claude 橙 + 霓虹青） */
export const MOOD_GLOW: Record<Mood, [string, string]> = {
  happy: [COLORS.claude, COLORS.brand],
  worried: [COLORS.warn, "#FF6B3D"],
  sleepy: [COLORS.danger, "#8B5CF6"],
  confused: ["#94A3B8", "#475569"],
};

/** 终端风状态码 */
export const MOOD_CODE: Record<Mood, string> = {
  happy: "NOMINAL",
  worried: "HIGH LOAD",
  sleepy: "DEPLETED",
  confused: "NO SIGNAL",
};

export const MOOD_TEXT: Record<Mood, string> = {
  happy: "额度充足",
  worried: "有点吃紧",
  sleepy: "额度耗尽",
  confused: "暂无数据",
};
