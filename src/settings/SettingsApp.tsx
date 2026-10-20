import {
  Alert,
  Button,
  ConfigProvider,
  Empty,
  Space,
  Spin,
  Tabs,
  Typography,
  theme,
  Tooltip,
} from "antd";
import zhCN from "antd/locale/zh_CN";
import {
  EyeInvisibleOutlined,
  EyeOutlined,
  ImportOutlined,
  PlusOutlined,
  ReloadOutlined,
} from "@ant-design/icons";

import { api } from "../api/commands";
import type { Snapshot } from "../api/types";
import { Masked } from "../components/Masked";
import { Mascot } from "../components/Mascot";
import { useAction, useNow, useSnapshot } from "../hooks/useSnapshot";
import { COLORS } from "../theme";
import { formatTime, moodOf } from "../utils/format";
import { AccountCard } from "./AccountCard";
import { HistoryPanel } from "./HistoryPanel";
import { LoginModal } from "./LoginModal";
import { RulesPanel } from "./RulesPanel";
import "./settings.css";

export function SettingsApp() {
  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: theme.darkAlgorithm,
        token: {
          colorPrimary: COLORS.brand,
          colorInfo: COLORS.brand,
          colorSuccess: COLORS.ok,
          colorWarning: COLORS.warn,
          colorError: COLORS.danger,
          colorBgBase: "#070a12",
          colorBgContainer: "#0d1220",
          colorBorder: "#1e2638",
          borderRadius: 8,
          fontFamily: `"Inter", "PingFang SC", "Microsoft YaHei UI", system-ui, sans-serif`,
        },
      }}
    >
      <div className="settings">
        <Content />
      </div>
    </ConfigProvider>
  );
}

function Content() {
  const snap = useSnapshot();
  const now = useNow();
  const action = useAction();

  if (!snap) {
    return (
      <div className="settings__loading">
        <Spin />
      </div>
    );
  }
  const active = snap.accounts.find((a) => a.isActive);

  return (
    <>
      <header className="settings__header">
        <Mascot mood={moodOf(active, snap.settings, now)} size={52} />
        <div className="settings__heading">
          <h1 className="settings__title">
            CLAUDE<span>::</span>AUTO
          </h1>
          <Typography.Text type="secondary">
            {snap.accounts.length} 个账号 · 当前{" "}
            {active ? (
              <Masked text={active.label} masked={snap.settings.privacyMode} revealable />
            ) : (
              "未纳管"
            )}{" "}
            · 上次刷新 {formatTime(snap.lastPollAt)}
          </Typography.Text>
        </div>
        <Space>
          <Tooltip
            title={snap.settings.privacyMode ? "隐私模式：已遮挡账号名" : "隐私模式：已关闭"}
          >
            <Button
              icon={snap.settings.privacyMode ? <EyeInvisibleOutlined /> : <EyeOutlined />}
              type={snap.settings.privacyMode ? "primary" : "default"}
              ghost={snap.settings.privacyMode}
              onClick={() =>
                void action.run("privacy", () =>
                  api.updateSettings({ ...snap.settings, privacyMode: !snap.settings.privacyMode }),
                )
              }
            />
          </Tooltip>
          <Button icon={<ReloadOutlined />} onClick={() => void api.refreshNow()}>
            刷新
          </Button>
          <Button
            icon={<PlusOutlined />}
            type="primary"
            onClick={() => void action.run("login", api.startLogin)}
          >
            添加账号
          </Button>
        </Space>
      </header>

      <Banners snap={snap} action={action} />
      <LoginModal url={snap.loginUrl} proxy={snap.proxy} privacy={snap.settings.privacyMode} />

      <Tabs
        className="settings__tabs"
        items={[
          {
            key: "accounts",
            label: "账号",
            children: <AccountsTab snap={snap} now={now} action={action} />,
          },
          {
            key: "rules",
            label: "自动切换",
            children: (
              <RulesPanel
                key={JSON.stringify(snap.settings)}
                settings={snap.settings}
                cliConfigDir={snap.cliConfigDir}
                proxy={snap.proxy}
                saving={action.busy === "settings"}
                onSave={(s) => void action.run("settings", () => api.updateSettings(s))}
              />
            ),
          },
          {
            key: "history",
            label: `切换记录${snap.history.length ? ` (${snap.history.length})` : ""}`,
            children: <HistoryPanel history={snap.history} privacy={snap.settings.privacyMode} />,
          },
        ]}
      />
    </>
  );
}

type Action = ReturnType<typeof useAction>;

function Banners({ snap, action }: { snap: Snapshot; action: Action }) {
  const managedActive = snap.activeId !== null;
  return (
    <div className="settings__banners">
      {action.error && (
        <Alert
          type="error"
          showIcon
          closable
          onClose={action.clearError}
          title={`操作失败（${action.error.code}）`}
          description={
            <Typography.Paragraph copyable style={{ marginBottom: 0 }}>
              {action.error.message}
            </Typography.Paragraph>
          }
        />
      )}
      {snap.lastError && (
        <Alert
          type="warning"
          showIcon
          title="上次轮询出错"
          description={
            <Typography.Paragraph copyable style={{ marginBottom: 0 }}>
              {snap.lastError}
            </Typography.Paragraph>
          }
        />
      )}
      {snap.pending && (
        <Alert
          type={snap.pending.toId ? "info" : "error"}
          showIcon
          title={
            <>
              <Masked text={snap.pending.fromLabel} masked={snap.settings.privacyMode} revealable />{" "}
              已达{snap.pending.reasonLabel}
            </>
          }
          description={
            snap.pending.toId ? (
              <>
                建议切换到{" "}
                <Masked
                  text={snap.pending.toLabel ?? ""}
                  masked={snap.settings.privacyMode}
                  revealable
                />
              </>
            ) : (
              "所有账号均已耗尽，等待窗口重置"
            )
          }
          action={
            <Space orientation="vertical">
              {snap.pending.toId && (
                <Button
                  type="primary"
                  size="small"
                  loading={action.busy === "apply"}
                  onClick={() => void action.run("apply", api.applyPending)}
                >
                  立即切换
                </Button>
              )}
              <Button size="small" onClick={() => void api.dismissPending()}>
                忽略
              </Button>
            </Space>
          }
        />
      )}
      {snap.cliLoggedIn && !managedActive && (
        <Alert
          type="info"
          showIcon
          title="CLI 当前登录的账号尚未纳入管理"
          action={
            <Button
              size="small"
              icon={<ImportOutlined />}
              loading={action.busy === "import"}
              onClick={() => void action.run("import", api.importCurrent)}
            >
              导入
            </Button>
          }
        />
      )}
    </div>
  );
}

function AccountsTab({ snap, now, action }: { snap: Snapshot; now: number; action: Action }) {
  if (snap.accounts.length === 0) {
    return (
      <Empty description="还没有账号" style={{ padding: "48px 0" }}>
        <Space>
          {snap.cliLoggedIn && (
            <Button
              icon={<ImportOutlined />}
              onClick={() => void action.run("import", api.importCurrent)}
            >
              导入 CLI 当前账号
            </Button>
          )}
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => void action.run("login", api.startLogin)}
          >
            登录新账号
          </Button>
        </Space>
      </Empty>
    );
  }
  return (
    <div className="settings__grid">
      {snap.accounts.map((a) => (
        <AccountCard
          key={a.id}
          account={a}
          settings={snap.settings}
          now={now}
          busy={action.busy}
          onSwitch={() => void action.run(`switch:${a.id}`, () => api.switchAccount(a.id))}
          onRemove={() => void action.run(`remove:${a.id}`, () => api.removeAccount(a.id))}
          onRename={(label) =>
            void action.run(`rename:${a.id}`, () => api.renameAccount(a.id, label))
          }
        />
      ))}
    </div>
  );
}
