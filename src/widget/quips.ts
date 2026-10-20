import type { PendingView } from "../api/types";
import type { Mood } from "../utils/format";

export interface QuipContext {
  mood: Mood;
  /** 5h 剩余百分比 */
  fiveHourLeft: number | null;
  /** 周剩余百分比 */
  weekLeft: number | null;
  /** 5h 重置倒计时（已格式化） */
  fiveHourReset: string;
  pending: PendingView | null;
  privacy: boolean;
  /** 登录即将过期账号中最少的剩余天数（无则 null） */
  loginExpiringDays: number | null;
}

const pct = (v: number | null) => (v === null ? "--" : `${Math.round(v)}%`);

/** 宠物气泡台词：按心情给出一组轮播文案。隐私模式下绝不出现账号名。 */
export function quipsFor(ctx: QuipContext): string[] {
  if (ctx.pending) {
    const who = ctx.privacy ? "下一位" : (ctx.pending.toLabel ?? "下一位");
    return ctx.pending.toId
      ? [`${ctx.pending.reasonLabel}到顶了！`, `换 ${who} 上场？`]
      : ["全员没电了…", `等 ${ctx.fiveHourReset} 后再战`];
  }
  const lines = moodQuips(ctx);
  if (ctx.loginExpiringDays !== null) {
    const d = ctx.loginExpiringDays;
    lines.unshift(d < 0 ? "有账号登录过期了！" : `有账号 ${d} 天后要重新登录`);
  }
  return lines;
}

function moodQuips(ctx: QuipContext): string[] {
  switch (ctx.mood) {
    case "happy":
      return [
        "额度充足，冲！",
        `5H 还剩 ${pct(ctx.fiveHourLeft)}`,
        "今天写点什么？",
        `本周还剩 ${pct(ctx.weekLeft)}`,
        "Token 管够 ✻",
      ];
    case "worried":
      return [`5H 只剩 ${pct(ctx.fiveHourLeft)}…`, "省着点用！", `本周剩 ${pct(ctx.weekLeft)}`];
    case "sleepy":
      return ["没电了 zZ", `${ctx.fiveHourReset} 后叫醒我`];
    case "confused":
      return ["信号丢失…", "检查一下代理？", "还没有数据"];
  }
}
