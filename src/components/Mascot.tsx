import { useId } from "react";

import type { Mood } from "../utils/format";
import "./mascot.css";

interface Props {
  mood: Mood;
  size?: number;
  /** 关闭待机动画（列表里的小头像用） */
  still?: boolean;
}

/** 像素格边长（viewBox 单位）；身体按 14×8 格绘制 */
const C = 6;
const X0 = 18;
const Y0 = 46;

const ORANGE = "#D97757";
const ORANGE_HI = "#EC9A7A";
const ORANGE_LO = "#B65A3C";
const INK = "#1B1210";

/** 网格坐标 → 像素方块 */
function px(col: number, row: number, w = 1, h = 1, fill = INK) {
  return (
    <rect
      key={`${col}-${row}-${w}-${h}-${fill}`}
      x={X0 + col * C}
      y={Y0 + row * C}
      width={w * C}
      height={h * C}
      fill={fill}
    />
  );
}

/** 各心情的眼睛（网格坐标） */
function Eyes({ mood }: { mood: Mood }) {
  switch (mood) {
    case "happy":
      return (
        <>
          {px(4, 1, 1, 2)}
          {px(9, 1, 1, 2)}
        </>
      );
    case "worried":
      // > <
      return (
        <>
          {px(3, 1)}
          {px(4, 2)}
          {px(3, 3)}
          {px(10, 1)}
          {px(9, 2)}
          {px(10, 3)}
        </>
      );
    case "sleepy":
      return (
        <>
          <rect x={X0 + 3 * C} y={Y0 + 2.4 * C} width={2 * C} height={C * 0.6} fill={INK} />
          <rect x={X0 + 9 * C} y={Y0 + 2.4 * C} width={2 * C} height={C * 0.6} fill={INK} />
        </>
      );
    case "confused":
      return (
        <>
          {px(4, 1, 1, 2)}
          {px(9, 2)}
        </>
      );
  }
}

/** Claude 风格星芒：8 道长短交替的圆头光线 */
function Spark({ color }: { color: string }) {
  return (
    <g>
      {Array.from({ length: 8 }, (_, i) => {
        const a = (i * Math.PI) / 4;
        const len = i % 2 === 0 ? 11.5 : 8;
        return (
          <line
            key={i}
            x1={0}
            y1={0}
            x2={Math.cos(a) * len}
            y2={Math.sin(a) * len}
            stroke={color}
            strokeWidth={3.6}
            strokeLinecap="round"
          />
        );
      })}
    </g>
  );
}

/** 像素问号（5×7） */
const QUESTION = ["01110", "10001", "00001", "00110", "00100", "00000", "00100"];

/**
 * Claude 像素小生物：橙色方块身体 + 星芒头饰。
 * 心情决定表情与动作：开心踏步挥手、紧张发抖冒汗、耗尽趴睡、无数据东张西望。
 */
export function Mascot({ mood, size = 96, still = false }: Props) {
  const uid = useId().replace(/:/g, "");
  const glowId = `glow-${uid}`;
  const sparkColor = mood === "worried" ? "#FFB020" : ORANGE;

  return (
    <svg
      className={`claw claw--${mood}${still ? " claw--still" : ""}`}
      width={size}
      height={size}
      viewBox="0 0 120 120"
      shapeRendering="crispEdges"
      role="img"
      aria-label={`Claude 宠物：${mood}`}
    >
      <defs>
        <filter id={glowId} x="-60%" y="-60%" width="220%" height="220%">
          <feGaussianBlur stdDeviation="2.2" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {/* 地面投影 */}
      <rect
        className="claw__shadow"
        x={30}
        y={100}
        width={60}
        height={4}
        rx={2}
        fill="#000"
        opacity={0.35}
      />

      {/* 头顶：星芒 / 问号（位置由 CSS 动画的 translate 决定） */}
      <g className="claw__crest">
        {mood === "confused" ? (
          <g fill="#94A3B8" transform="translate(-7.5 -10.5)">
            {QUESTION.flatMap((row, r) =>
              [...row].map((b, c) =>
                b === "1" ? (
                  <rect key={`${r}-${c}`} x={c * 3} y={r * 3} width={3} height={3} />
                ) : null,
              ),
            )}
          </g>
        ) : (
          <g className="claw__spark" filter={`url(#${glowId})`} shapeRendering="geometricPrecision">
            <Spark color={sparkColor} />
          </g>
        )}
      </g>

      <g className="claw__all">
        {/* 腿：两组交替踏步 */}
        <g className="claw__legs-a">
          {px(3, 6, 1, 2, ORANGE_LO)}
          {px(8, 6, 1, 2, ORANGE_LO)}
        </g>
        <g className="claw__legs-b">
          {px(5, 6, 1, 2, ORANGE_LO)}
          {px(10, 6, 1, 2, ORANGE_LO)}
        </g>

        <g className="claw__torso">
          {/* 手臂 */}
          <g className="claw__arm-l">{px(0, 2, 2, 2, ORANGE)}</g>
          <g className="claw__arm-r">{px(12, 2, 2, 2, ORANGE)}</g>
          {/* 身体 + 像素高光/阴影 */}
          {px(2, 0, 10, 6, ORANGE)}
          {px(3, 0, 8, 1, ORANGE_HI)}
          {px(2, 5, 10, 1, ORANGE_LO)}

          <g className="claw__look">
            <g className="claw__eyes">
              <Eyes mood={mood} />
            </g>
          </g>
        </g>
      </g>

      {/* 心情特效 */}
      {mood === "happy" && (
        <g className="claw__sparkles" fill="#FFD7A8">
          <rect className="s1" x={14} y={58} width={3} height={3} />
          <rect className="s2" x={102} y={52} width={3} height={3} />
          <rect className="s3" x={96} y={70} width={2.5} height={2.5} />
        </g>
      )}
      {mood === "worried" && (
        <g className="claw__sweat" fill="#7DD3FC">
          <rect x={92} y={44} width={3} height={3} />
          <rect x={91} y={47} width={5} height={4} />
        </g>
      )}
      {mood === "sleepy" && (
        <g className="claw__zzz" fill="#A78BFA" fontFamily="Consolas, monospace" fontWeight={800}>
          <text x={90} y={42} fontSize={12}>
            Z
          </text>
          <text x={100} y={30} fontSize={9}>
            z
          </text>
        </g>
      )}
    </svg>
  );
}
