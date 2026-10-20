import { describe, expect, it } from "vitest";

import type { AccountView, Settings } from "../src/api/types";
import {
  effectiveUtil,
  formatCountdown,
  formatCountdownCompact,
  levelOf,
  moodOf,
  shortLabel,
  daysLeft,
} from "../src/utils/format";

const settings: Settings = {
  mode: "semi",
  fiveHourThreshold: 95,
  sevenDayThreshold: 90,
  pollSecs: 180,
  proxyMode: "cli",
  proxyUrl: "",
  privacyMode: true,
};

function account(fh: number, sd: number): AccountView {
  return {
    id: "a",
    label: "a",
    email: null,
    plan: "pro",
    isActive: true,
    tokenExpiresAt: null,
    usage: {
      fiveHour: { utilization: fh, resetsAt: 10_000 },
      sevenDay: { utilization: sd, resetsAt: 20_000 },
    },
    refreshExpiresAt: null,
    renewal: null,
    error: null,
    needsRelogin: false,
    checkedAt: null,
  };
}

describe("formatCountdown", () => {
  it("formats days, hours and minutes", () => {
    expect(formatCountdown(0 + (2 * 1440 + 3 * 60) * 60_000, 0)).toBe("2天3时");
    expect(formatCountdown((2 * 60 + 13) * 60_000, 0)).toBe("2时13分");
    expect(formatCountdown(30_000, 0)).toBe("1分");
    expect(formatCountdown(5, 10)).toBe("即将重置");
    expect(formatCountdown(null, 0)).toBe("--");
  });
});

describe("formatCountdownCompact", () => {
  it("pads for HUD alignment", () => {
    expect(formatCountdownCompact((3 * 1440 + 4 * 60) * 60_000, 0)).toBe("3d04h");
    expect(formatCountdownCompact((2 * 60 + 3) * 60_000, 0)).toBe("2h03m");
    expect(formatCountdownCompact(8 * 60_000, 0)).toBe("08m");
    expect(formatCountdownCompact(5, 10)).toBe("00m");
  });
});

describe("daysLeft", () => {
  it("floors like the backend", () => {
    const day = 86_400_000;
    expect(daysLeft(3 * day + 5, 0)).toBe(3);
    expect(daysLeft(day - 1, 0)).toBe(0);
    expect(daysLeft(0, 1)).toBe(-1);
  });
});

describe("shortLabel", () => {
  it("keeps first two characters only", () => {
    expect(shortLabel("alice@example.com")).toBe("al");
    expect(shortLabel("工作号")).toBe("工作");
    expect(shortLabel("a")).toBe("a");
  });
});

describe("effectiveUtil", () => {
  it("treats passed reset as recovered", () => {
    expect(effectiveUtil({ utilization: 99, resetsAt: 100 }, 200)).toBe(0);
    expect(effectiveUtil({ utilization: 99, resetsAt: 300 }, 200)).toBe(99);
    expect(effectiveUtil(null, 0)).toBeNull();
  });
});

describe("levelOf / moodOf", () => {
  it("grades against threshold", () => {
    expect(levelOf(96, 95)).toBe("danger");
    expect(levelOf(80, 95)).toBe("warn");
    expect(levelOf(10, 95)).toBe("ok");
    expect(levelOf(null, 95)).toBe("unknown");
  });

  it("uses the tighter window", () => {
    expect(moodOf(account(10, 10), settings, 0)).toBe("happy");
    expect(moodOf(account(10, 75), settings, 0)).toBe("worried");
    expect(moodOf(account(99, 10), settings, 0)).toBe("sleepy");
    // 5h 已过重置点 → 恢复
    expect(moodOf(account(99, 10), settings, 15_000)).toBe("happy");
    expect(moodOf(undefined, settings, 0)).toBe("confused");
  });
});
