import { Button, Popconfirm, Progress, Tag, Tooltip, Typography } from "antd";
import { DeleteOutlined, SwapOutlined, WarningOutlined } from "@ant-design/icons";

import type { AccountView, Settings, UsageWindow } from "../api/types";
import { Masked, useTimedReveal } from "../components/Masked";
import { Mascot } from "../components/Mascot";
import { LEVEL_COLOR } from "../theme";
import {
  effectiveUtil,
  formatCountdown,
  formatPct,
  formatTime,
  levelOf,
  moodOf,
  planLabel,
} from "../utils/format";

interface Props {
  account: AccountView;
  settings: Settings;
  now: number;
  busy: string | null;
  onSwitch: () => void;
  onRemove: () => void;
  onRename: (label: string) => void;
}

export function AccountCard({
  account: a,
  settings,
  now,
  busy,
  onSwitch,
  onRemove,
  onRename,
}: Props) {
  const mood = moodOf(a, settings, now);
  const { revealed, setRevealed } = useTimedReveal();
  const hidden = settings.privacyMode && !revealed;
  return (
    <div className={`acc-card${a.isActive ? " acc-card--active" : ""}`}>
      <div className="acc-card__head">
        <Mascot mood={mood} size={44} still={!a.isActive} />
        <div className="acc-card__id">
          {hidden ? (
            <span className="acc-card__reveal" title="点击显示" onClick={() => setRevealed(true)}>
              <span className="acc-card__label">
                <Masked text={a.label} masked />
              </span>
              <span className="acc-card__email">
                <Masked text={a.email ?? a.label} masked />
              </span>
            </span>
          ) : (
            <>
              <Typography.Text
                strong
                className="acc-card__label"
                editable={{ onChange: onRename, maxLength: 40, tooltip: "重命名" }}
              >
                {a.label}
              </Typography.Text>
              <Typography.Text type="secondary" className="acc-card__email" ellipsis>
                {a.email ?? "邮箱未知"}
              </Typography.Text>
            </>
          )}
        </div>
        <div className="acc-card__tags">
          <Tag variant="filled">{planLabel(a.plan)}</Tag>
          {a.isActive && (
            <Tag variant="filled" color="cyan">
              使用中
            </Tag>
          )}
        </div>
      </div>

      <UsageRow
        label="5 小时"
        window={a.usage?.fiveHour}
        threshold={settings.fiveHourThreshold}
        now={now}
      />
      <UsageRow
        label="每周"
        window={a.usage?.sevenDay}
        threshold={settings.sevenDayThreshold}
        now={now}
      />

      {a.error && (
        <Typography.Paragraph
          type={a.needsRelogin ? "danger" : "warning"}
          className="acc-card__error"
          copyable={{ text: a.error, tooltips: ["复制错误", "已复制"] }}
        >
          <WarningOutlined /> {a.needsRelogin ? "凭据失效，请删除后重新添加。" : ""}
          {a.error}
        </Typography.Paragraph>
      )}

      <div className="acc-card__foot">
        <Typography.Text type="secondary" className="acc-card__meta">
          更新于 {formatTime(a.checkedAt)}
        </Typography.Text>
        <div className="acc-card__actions">
          <Popconfirm
            title="移除该账号？"
            description="只从本工具移除，不会登出 CLI。"
            okText="移除"
            cancelText="取消"
            onConfirm={onRemove}
          >
            <Tooltip title="移除">
              <Button type="text" size="small" icon={<DeleteOutlined />} />
            </Tooltip>
          </Popconfirm>
          {!a.isActive && (
            <Button
              size="small"
              type="primary"
              icon={<SwapOutlined />}
              loading={busy === `switch:${a.id}`}
              disabled={busy !== null || a.needsRelogin}
              onClick={onSwitch}
            >
              切换到此账号
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

function UsageRow(props: {
  label: string;
  window: UsageWindow | null | undefined;
  threshold: number;
  now: number;
}) {
  const util = effectiveUtil(props.window, props.now);
  const color = LEVEL_COLOR[levelOf(util, props.threshold)];
  return (
    <div className="usage-row">
      <span className="usage-row__label">{props.label}</span>
      <Progress
        className="usage-row__bar"
        percent={util ?? 0}
        showInfo={false}
        strokeColor={color}
        size="small"
      />
      <span className="usage-row__pct">{formatPct(util)}</span>
      <span className="usage-row__reset">
        {formatCountdown(props.window?.resetsAt, props.now)}后重置
      </span>
    </div>
  );
}
