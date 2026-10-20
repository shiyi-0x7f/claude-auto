import { describe, expect, it } from "vitest";

import type { PendingView } from "../src/api/types";
import { quipsFor, type QuipContext } from "../src/widget/quips";

const base: QuipContext = {
  mood: "happy",
  fiveHourLeft: 47.4,
  weekLeft: 90,
  fiveHourReset: "2h13m",
  pending: null,
  privacy: true,
};

const pending: PendingView = {
  fromId: "a",
  fromLabel: "alice@example.com",
  toId: "b",
  toLabel: "bob@example.com",
  reason: "fiveHour",
  reasonLabel: "5 小时限额",
};

describe("quipsFor", () => {
  it("shows remaining quota for happy mood", () => {
    expect(quipsFor(base)).toContain("5H 还剩 47%");
  });

  it("never leaks account names in privacy mode", () => {
    const lines = quipsFor({ ...base, pending });
    expect(lines.join("")).not.toContain("bob");
    expect(lines.join("")).not.toContain("alice");
    expect(lines).toContain("换 下一位 上场？");
  });

  it("names the target when privacy is off", () => {
    expect(quipsFor({ ...base, pending, privacy: false })).toContain("换 bob@example.com 上场？");
  });

  it("handles all accounts exhausted", () => {
    const lines = quipsFor({ ...base, pending: { ...pending, toId: null, toLabel: null } });
    expect(lines).toContain("等 2h13m 后再战");
  });
});
