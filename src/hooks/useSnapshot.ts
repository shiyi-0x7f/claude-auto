import { useCallback, useEffect, useState } from "react";

import { api, onSnapshot, toApiError } from "../api/commands";
import type { ApiError, Snapshot } from "../api/types";

/** 订阅后端推送的状态快照；首帧主动拉取一次。 */
export function useSnapshot(): Snapshot | null {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  useEffect(() => {
    let disposed = false;
    const unlisten = onSnapshot((s) => {
      if (!disposed) setSnap(s);
    });
    api
      .getSnapshot()
      .then((s) => {
        if (!disposed) setSnap(s);
      })
      .catch((e: unknown) => console.error("get_snapshot failed", e));
    return () => {
      disposed = true;
      void unlisten.then((f) => f());
    };
  }, []);
  return snap;
}

/** 每隔 intervalMs 更新一次的当前时间，用于倒计时。 */
export function useNow(intervalMs = 30_000): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(t);
  }, [intervalMs]);
  return now;
}

/**
 * 包装后端调用：记录进行中的动作与最近一次错误。
 * 错误常驻展示（可复制），由用户手动关闭，不用一闪而过的 toast。
 */
export function useAction() {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const run = useCallback(async (key: string, fn: () => Promise<unknown>) => {
    setBusy(key);
    try {
      await fn();
      setError(null);
      return true;
    } catch (e) {
      setError(toApiError(e));
      return false;
    } finally {
      setBusy(null);
    }
  }, []);
  return { busy, error, clearError: () => setError(null), run };
}
