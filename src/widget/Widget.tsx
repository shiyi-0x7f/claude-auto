import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CloseOutlined, ReloadOutlined, SettingOutlined } from "@ant-design/icons";

import { api } from "../api/commands";
import type { AccountView, Settings, UsageWindow } from "../api/types";
import { Mascot } from "../components/Mascot";
import { useAction, useNow, useSnapshot } from "../hooks/useSnapshot";
import { LEVEL_COLOR, MOOD_CODE, MOOD_GLOW, MOOD_TEXT } from "../theme";
import {
  effectiveUtil,
  formatCountdownCompact,
  levelOf,
  moodOf,
  daysLeft,
  planLabel,
  RELOGIN_WARN_DAYS,
  shortLabel,
} from "../utils/format";
import { quipsFor } from "./quips";
import "./widget.css";

const SEGMENTS = 18;
const TYPE_MS = 70;
const LINE_HOLD_MS = 6000;

/** 按住非交互区域拖动窗口（按钮与宠物本身可点击，不触发拖动） */
function onDragStart(e: React.MouseEvent) {
  if (e.button !== 0 || (e.target as HTMLElement).closest("button, .pet")) return;
  void getCurrentWindow().startDragging();
}

/** 打字机轮播：用单一节拍计数推导当前句子与已打出的字数（纯推导，文案变化无需重置状态）。 */
function useTypewriter(lines: string[]): string {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const t = window.setInterval(() => setTick((v) => v + 1), TYPE_MS);
    return () => window.clearInterval(t);
  }, []);
  if (lines.length === 0) return "";
  const perLine = Math.round(LINE_HOLD_MS / TYPE_MS);
  const line = lines[Math.floor(tick / perLine) % lines.length];
  return [...line].slice(0, tick % perLine).join("");
}

export function Widget() {
  const snap = useSnapshot();
  const now = useNow(15_000);
  const { busy, error, clearError, run } = useAction();
  const [pinned, setPinned] = useState(false);

  const active = snap?.accounts.find((a) => a.isActive);
  const settings = snap?.settings;
  const mood = settings ? moodOf(active, settings, now) : "confused";
  const fhUtil = effectiveUtil(active?.usage?.fiveHour, now);
  const sdUtil = effectiveUtil(active?.usage?.sevenDay, now);
  const lines = quipsFor({
    mood,
    fiveHourLeft: fhUtil === null ? null : 100 - fhUtil,
    weekLeft: sdUtil === null ? null : 100 - sdUtil,
    fiveHourReset: formatCountdownCompact(active?.usage?.fiveHour?.resetsAt, now),
    pending: snap?.pending ?? null,
    privacy: settings?.privacyMode ?? true,
    loginExpiringDays: minLoginDays(snap?.accounts ?? [], now),
  });
  const typed = useTypewriter(error ? [`ERR ${error.code}`, error.message] : lines);

  if (!snap || !settings) return <div className="stage" />;
  const privacy = settings.privacyMode;
  const pending = snap.pending;
  const [glow, alt] = MOOD_GLOW[mood];
  const name = (label: string) => (privacy ? shortLabel(label) : label);
  // 气泡默认隐藏：悬停时显示（CSS），点击宠物固定；出错时自动显示
  const bubbleOn = pinned || error !== null;

  return (
    <div
      className={`stage stage--${mood}${bubbleOn ? " stage--bubble" : ""}`}
      style={{ ["--glow" as string]: glow, ["--alt" as string]: alt }}
      onMouseDown={onDragStart}
      onDoubleClick={() => void api.openMain()}
    >
      <div
        className="pet"
        title={pinned ? "点击收起对话" : "点击固定对话"}
        onClick={() => setPinned((v) => !v)}
      >
        <Mascot mood={mood} size={92} />
      </div>

      <i className={`bubble-tail${error ? " bubble-tail--error" : ""}`} />
      <div
        className={`bubble${error ? " bubble--error" : ""}`}
        onClick={error ? clearError : undefined}
        title={error ? "点击关闭错误" : MOOD_TEXT[mood]}
      >
        {typed}
        <i className="bubble__caret" />
      </div>

      <div className="panel-shadow" />
      <div className="panel">
        <div className="panel__inner">
          <div className="panel__head">
            <span
              className="panel__name"
              title={privacy ? undefined : (active?.email ?? undefined)}
            >
              <span className="panel__proto">claude://</span>
              {active ? name(active.label) : snap.cliLoggedIn ? "unmanaged" : "offline"}
            </span>
            {active && <span className="panel__badge">{planLabel(active.plan).toUpperCase()}</span>}
          </div>

          <Bar
            icon="heart"
            label="5H"
            window={active?.usage?.fiveHour}
            threshold={settings.fiveHourThreshold}
            now={now}
          />
          <Bar
            icon="bolt"
            label="7D"
            window={active?.usage?.sevenDay}
            threshold={settings.sevenDayThreshold}
            now={now}
          />

          <div className="panel__foot">
            {pending?.toId ? (
              <button
                className="swap"
                disabled={busy !== null}
                onClick={() => void run("apply", api.applyPending)}
              >
                {busy === "apply" ? "SWAPPING…" : <>⇄ 换人上场 {name(pending.toLabel ?? "")}</>}
              </button>
            ) : (
              <>
                <Party accounts={snap.accounts} settings={settings} now={now} />
                <span className="panel__code">[{MOOD_CODE[mood]}]</span>
              </>
            )}
          </div>
        </div>
      </div>

      <div className="tools">
        <button title="立即刷新" onClick={() => void api.refreshNow()}>
          <ReloadOutlined />
        </button>
        <button title="打开设置" onClick={() => void api.openMain()}>
          <SettingOutlined />
        </button>
        <button title="隐藏挂件（托盘可恢复）" onClick={() => void getCurrentWindow().hide()}>
          <CloseOutlined />
        </button>
      </div>
    </div>
  );
}

/** 像素小图标（7×7 位图） */
const ICONS: Record<"heart" | "bolt", string[]> = {
  heart: ["0000000", "0110110", "1111111", "1111111", "0111110", "0011100", "0001000"],
  bolt: ["0000110", "0001100", "0011000", "0111110", "0001100", "0011000", "0110000"],
};

function PixelIcon({ name, color }: { name: "heart" | "bolt"; color: string }) {
  return (
    <svg className="pixicon" viewBox="0 0 7 7" width={14} height={14} shapeRendering="crispEdges">
      {ICONS[name].flatMap((row, r) =>
        [...row].map((b, c) =>
          b === "1" ? (
            <rect key={`${r}-${c}`} x={c} y={r} width={1} height={1} fill={color} />
          ) : null,
        ),
      )}
    </svg>
  );
}

/** 游戏血条：显示「剩余」额度，用得越多越短 */
function Bar(props: {
  icon: "heart" | "bolt";
  label: string;
  window: UsageWindow | null | undefined;
  threshold: number;
  now: number;
}) {
  const util = effectiveUtil(props.window, props.now);
  const left = util === null ? null : Math.max(100 - util, 0);
  const color = LEVEL_COLOR[levelOf(util, props.threshold)];
  const lit = left === null ? 0 : Math.ceil((left / 100) * SEGMENTS);
  return (
    <div className="bar" style={{ ["--c" as string]: color }}>
      <PixelIcon name={props.icon} color={color} />
      <span className="bar__label">{props.label}</span>
      <span className="bar__track">
        {Array.from({ length: SEGMENTS }, (_, i) => (
          <i key={i} className={i < lit ? "on" : undefined} />
        ))}
      </span>
      <span className="bar__pct">{left === null ? "--" : `${Math.round(left)}%`}</span>
      <span className="bar__reset">
        ↻{formatCountdownCompact(props.window?.resetsAt, props.now)}
      </span>
    </div>
  );
}

/** 账号登录剩余天数：需重新登录视为已过期 */
function loginDays(a: AccountView, now: number): number | null {
  if (a.needsRelogin) return -1;
  return a.refreshExpiresAt === null ? null : daysLeft(a.refreshExpiresAt, now);
}

/** 登录即将过期（< RELOGIN_WARN_DAYS 天）的账号中最少的剩余天数 */
function minLoginDays(accounts: AccountView[], now: number): number | null {
  const days = accounts
    .map((a) => loginDays(a, now))
    .filter((d): d is number => d !== null && d < RELOGIN_WARN_DAYS);
  return days.length ? Math.min(...days) : null;
}

/** 队伍栏：每个账号一个像素小头像，颜色表示额度状态；在场账号头顶有 ▼，登录将过期的显示 ! */
function Party(props: { accounts: AccountView[]; settings: Settings; now: number }) {
  return (
    <span className="party">
      {props.accounts.map((a) => {
        const mood = moodOf(a, props.settings, props.now);
        const color = a.error !== null ? LEVEL_COLOR.unknown : MOOD_GLOW[mood][0];
        const d = loginDays(a, props.now);
        const alert = d !== null && d < RELOGIN_WARN_DAYS;
        return (
          <i
            key={a.id}
            className={`party__pet${a.isActive ? " party__pet--active" : ""}${alert ? " party__pet--alert" : ""}`}
            style={{ ["--c" as string]: color }}
            title={props.settings.privacyMode ? MOOD_TEXT[mood] : `${a.label} · ${MOOD_TEXT[mood]}`}
          />
        );
      })}
    </span>
  );
}
