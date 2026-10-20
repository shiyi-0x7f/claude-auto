import { useState } from "react";
import {
  Button,
  Card,
  Col,
  Input,
  InputNumber,
  Row,
  Segmented,
  Slider,
  Space,
  Switch,
  Typography,
} from "antd";

import type { ProxyMode, Settings, SwitchMode } from "../api/types";

interface Props {
  settings: Settings;
  cliConfigDir: string;
  /** 当前生效代理（已脱敏） */
  proxy: string | null;
  saving: boolean;
  onSave: (s: Settings) => void;
}

const MODE_OPTIONS: { label: string; value: SwitchMode }[] = [
  { label: "半自动 · 提示后确认", value: "semi" },
  { label: "全自动 · 直接切换", value: "auto" },
];

const PROXY_OPTIONS: { label: string; value: ProxyMode }[] = [
  { label: "跟随 CLI 配置", value: "cli" },
  { label: "自定义", value: "custom" },
  { label: "不使用", value: "none" },
];

export function RulesPanel({ settings, cliConfigDir, proxy, saving, onSave }: Props) {
  const [draft, setDraft] = useState(settings);
  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);
  const set = (patch: Partial<Settings>) => setDraft((d) => ({ ...d, ...patch }));

  return (
    <Space orientation="vertical" size={16} style={{ width: "100%" }}>
      <Card title="切换方式" size="small">
        <Segmented
          block
          options={MODE_OPTIONS}
          value={draft.mode}
          onChange={(v) => set({ mode: v })}
        />
        <Typography.Paragraph type="secondary" style={{ marginTop: 12, marginBottom: 0 }}>
          {draft.mode === "semi"
            ? "当前账号达到阈值时弹出系统通知，在挂件、托盘或本窗口一键确认切换。"
            : "当前账号达到阈值时自动切换到额度最充足的账号，并发送通知。"}
        </Typography.Paragraph>
      </Card>

      <Card title="触发阈值（已用百分比）" size="small">
        <ThresholdRow
          label="5 小时限额"
          hint="短窗口，约 5 小时滚动重置"
          value={draft.fiveHourThreshold}
          onChange={(v) => set({ fiveHourThreshold: v })}
        />
        <ThresholdRow
          label="周限额"
          hint="长窗口，7 天重置；周额度紧张的账号不会被选为切换目标"
          value={draft.sevenDayThreshold}
          onChange={(v) => set({ sevenDayThreshold: v })}
        />
      </Card>

      <Card title="监控" size="small">
        <Space align="center" wrap>
          <span>每</span>
          <InputNumber
            min={60}
            max={3600}
            step={30}
            value={draft.pollSecs}
            onChange={(v) => v !== null && set({ pollSecs: v })}
          />
          <span>秒查询一次所有账号额度</span>
        </Space>
        <Typography.Paragraph type="secondary" style={{ marginTop: 12, marginBottom: 0 }}>
          CLI 配置目录：
          <Typography.Text code copyable>
            {cliConfigDir}
          </Typography.Text>
        </Typography.Paragraph>
      </Card>

      <Card title="隐私" size="small">
        <Space align="center">
          <Switch checked={draft.privacyMode} onChange={(v) => set({ privacyMode: v })} />
          <span>隐私模式（防录屏）</span>
        </Space>
        <Typography.Paragraph type="secondary" style={{ marginTop: 12, marginBottom: 0 }}>
          开启后挂件与本窗口中的账号名、邮箱显示为马赛克，托盘提示与系统通知中只保留前两个字符。设置页点击马赛克可临时查看完整名称。
        </Typography.Paragraph>
      </Card>

      <Card title="网络代理" size="small">
        <Segmented
          block
          options={PROXY_OPTIONS}
          value={draft.proxyMode}
          onChange={(v) => set({ proxyMode: v })}
        />
        {draft.proxyMode === "custom" && (
          <Input
            style={{ marginTop: 12 }}
            placeholder="http://user:pass@host:port 或 socks5://host:port"
            value={draft.proxyUrl}
            onChange={(e) => set({ proxyUrl: e.target.value })}
          />
        )}
        <Typography.Paragraph type="secondary" style={{ marginTop: 12, marginBottom: 0 }}>
          {draft.proxyMode === "cli"
            ? "读取 CLI settings.json 中 env 的 HTTPS_PROXY / HTTP_PROXY / NO_PROXY。"
            : draft.proxyMode === "custom"
              ? "额度查询、令牌刷新和登录都走该代理。"
              : "所有请求直连。"}
          当前生效：{proxy ?? "直连"}
        </Typography.Paragraph>
      </Card>

      <Space>
        <Button type="primary" disabled={!dirty} loading={saving} onClick={() => onSave(draft)}>
          保存设置
        </Button>
        <Button disabled={!dirty} onClick={() => setDraft(settings)}>
          还原
        </Button>
      </Space>
    </Space>
  );
}

function ThresholdRow(props: {
  label: string;
  hint: string;
  value: number;
  onChange: (v: number) => void;
}) {
  return (
    <Row align="middle" gutter={16} style={{ marginBottom: 12 }}>
      <Col flex="120px">
        <Typography.Text strong>{props.label}</Typography.Text>
      </Col>
      <Col flex="auto">
        <Slider min={50} max={100} value={props.value} onChange={props.onChange} />
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {props.hint}
        </Typography.Text>
      </Col>
      <Col flex="96px">
        <InputNumber
          min={1}
          max={100}
          value={props.value}
          suffix="%"
          onChange={(v) => v !== null && props.onChange(v)}
        />
      </Col>
    </Row>
  );
}
