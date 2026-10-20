import { useState } from "react";
import { Alert, Button, Input, Modal, Space, Steps, Typography } from "antd";
import { CopyOutlined, ExportOutlined, ReloadOutlined } from "@ant-design/icons";
import { openUrl } from "@tauri-apps/plugin-opener";

import { api, toApiError } from "../api/commands";
import { Masked } from "../components/Masked";
import type { ApiError } from "../api/types";

interface Props {
  /** 授权链接；null 时弹窗关闭 */
  url: string | null;
  proxy: string | null;
  privacy: boolean;
}

/** 界面内登录：复制/打开授权链接 → 浏览器授权 → 粘贴回调页的授权码 */
export function LoginModal({ url, proxy, privacy }: Props) {
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [copied, setCopied] = useState(false);
  const [added, setAdded] = useState<string | null>(null);

  const reset = () => {
    setCode("");
    setError(null);
    setCopied(false);
  };

  const submit = async () => {
    setBusy(true);
    try {
      const label = await api.completeLogin(code);
      setAdded(label);
      reset();
    } catch (e) {
      setError(toApiError(e));
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
    } catch (e) {
      setError({ code: "CLIPBOARD", message: `复制失败，请手动选中链接复制：${String(e)}` });
    }
  };

  const open = async () => {
    if (!url) return;
    try {
      await openUrl(url);
    } catch (e) {
      setError({ code: "OPENER", message: `无法打开浏览器，请复制链接手动打开：${String(e)}` });
    }
  };

  return (
    <>
      <Modal
        open={url !== null}
        title="添加账号"
        width={560}
        mask={{ closable: false }}
        onCancel={() => {
          reset();
          void api.cancelLogin();
        }}
        footer={
          <Space>
            <Button
              icon={<ReloadOutlined />}
              onClick={() => {
                reset();
                void api.startLogin();
              }}
            >
              重新生成链接
            </Button>
            <Button
              type="primary"
              loading={busy}
              disabled={!code.includes("#")}
              onClick={() => void submit()}
            >
              添加账号
            </Button>
          </Space>
        }
      >
        <Steps
          orientation="vertical"
          size="small"
          current={code ? 1 : 0}
          items={[
            {
              title: "在浏览器中打开授权链接",
              description: (
                <Space orientation="vertical" style={{ width: "100%" }}>
                  <Input.TextArea
                    readOnly
                    value={url ?? ""}
                    autoSize={{ minRows: 2, maxRows: 3 }}
                    onFocus={(e) => e.target.select()}
                    style={{ fontSize: 12 }}
                  />
                  <Space>
                    <Button size="small" icon={<ExportOutlined />} onClick={() => void open()}>
                      在浏览器打开
                    </Button>
                    <Button size="small" icon={<CopyOutlined />} onClick={() => void copy()}>
                      {copied ? "已复制" : "复制链接"}
                    </Button>
                  </Space>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    用要添加的账号登录并授权。浏览器已登录了别的账号时，可用无痕窗口打开。
                  </Typography.Text>
                </Space>
              ),
            },
            {
              title: "粘贴授权码",
              description: (
                <Space orientation="vertical" style={{ width: "100%" }}>
                  <Input
                    placeholder="授权后页面显示的授权码，形如 xxxx#yyyy"
                    value={code}
                    onChange={(e) => {
                      setCode(e.target.value);
                      setError(null);
                    }}
                    onPressEnter={() => code.includes("#") && void submit()}
                    allowClear
                  />
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    换取令牌经由 {proxy ? `代理 ${proxy}` : "直连（未配置代理）"}
                  </Typography.Text>
                </Space>
              ),
            },
          ]}
        />
        {error && (
          <Alert
            type="error"
            showIcon
            title={`登录失败（${error.code}）`}
            description={
              <Typography.Paragraph copyable style={{ marginBottom: 0 }}>
                {error.message}
              </Typography.Paragraph>
            }
          />
        )}
      </Modal>
      <Modal
        open={added !== null}
        title="已添加账号"
        okText="好的"
        cancelButtonProps={{ style: { display: "none" } }}
        onOk={() => setAdded(null)}
        onCancel={() => setAdded(null)}
      >
        <Masked text={added ?? ""} masked={privacy} revealable /> 已加入账号库，额度会在几秒内刷新。
      </Modal>
    </>
  );
}
