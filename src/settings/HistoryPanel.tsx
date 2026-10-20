import { Empty, Timeline, Typography } from "antd";

import type { SwitchRecord } from "../api/types";
import { Masked } from "../components/Masked";
import { formatTime } from "../utils/format";

export function HistoryPanel({ history, privacy }: { history: SwitchRecord[]; privacy: boolean }) {
  if (history.length === 0) {
    return <Empty description="本次运行还没有切换记录" />;
  }
  return (
    <Timeline
      items={history.map((h) => ({
        color: h.trigger.startsWith("自动") ? "cyan" : "gray",
        children: (
          <>
            <Typography.Text strong>
              {h.fromLabel ? (
                <Masked text={h.fromLabel} masked={privacy} revealable />
              ) : (
                "未纳管账号"
              )}{" "}
              → <Masked text={h.toLabel} masked={privacy} revealable />
            </Typography.Text>
            <br />
            <Typography.Text type="secondary">
              {formatTime(h.at)} · {h.trigger}
            </Typography.Text>
          </>
        ),
      }))}
    />
  );
}
