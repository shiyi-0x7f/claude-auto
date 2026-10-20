import { useEffect, useState } from "react";

import "./masked.css";

interface Props {
  text: string;
  /** 是否遮挡（隐私模式） */
  masked: boolean;
  /** 允许点击临时显示（设置页用；挂件不允许） */
  revealable?: boolean;
  /** 临时显示后自动重新遮挡的毫秒数 */
  revealMs?: number;
}

const SHADES = ["#2a3550", "#3b4a6b", "#4c5d82", "#5f7299", "#33405e", "#46577a"];

/**
 * 由文本生成稳定的马赛克色块（同一名字图案不变）。
 * 列数随文本长度变化但有上下限，避免泄露精确长度。
 */
export function mosaicCells(text: string): { cols: number; cells: string[] } {
  let seed = 2166136261;
  for (const ch of text) seed = Math.imul(seed ^ ch.charCodeAt(0), 16777619) >>> 0;
  const cols = Math.min(Math.max(Math.round(text.length * 0.9), 6), 14);
  const cells: string[] = [];
  for (let i = 0; i < cols * 2; i++) {
    seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
    cells.push(SHADES[(seed >>> 16) % SHADES.length]);
  }
  return { cols, cells };
}

/** 临时显示：setRevealed(true) 后 ms 毫秒自动恢复遮挡 */
export function useTimedReveal(ms = 6000) {
  const [revealed, setRevealed] = useState(false);
  useEffect(() => {
    if (!revealed) return;
    const t = window.setTimeout(() => setRevealed(false), ms);
    return () => window.clearTimeout(t);
  }, [revealed, ms]);
  return { revealed, setRevealed };
}

/** 马赛克遮挡：遮挡时 DOM 里只有色块、不含真实文字（录屏/截图/开发者工具都看不到）。 */
export function Masked({ text, masked, revealable = false, revealMs = 6000 }: Props) {
  const { revealed, setRevealed } = useTimedReveal(revealMs);

  if (!masked || revealed) {
    return revealable && masked ? (
      <span className="masked-text" title="点击重新遮挡" onClick={() => setRevealed(false)}>
        {text}
      </span>
    ) : (
      <>{text}</>
    );
  }

  const { cols, cells } = mosaicCells(text);
  return (
    <span
      className={`mosaic${revealable ? " mosaic--revealable" : ""}`}
      style={{ ["--cols" as string]: cols }}
      role="img"
      aria-label="已隐藏的账号名"
      title={revealable ? "点击显示" : "隐私模式已开启"}
      onClick={
        revealable
          ? (e) => {
              e.stopPropagation();
              setRevealed(true);
            }
          : undefined
      }
    >
      {cells.map((c, i) => (
        <i key={i} style={{ background: c }} />
      ))}
    </span>
  );
}
