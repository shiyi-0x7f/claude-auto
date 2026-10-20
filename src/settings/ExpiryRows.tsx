import { Button, DatePicker, Tooltip, Typography } from "antd";
import { LoginOutlined, UndoOutlined } from "@ant-design/icons";
import dayjs from "dayjs";

import type { AccountView } from "../api/types";
import { daysLeft, formatDate, RELOGIN_WARN_DAYS } from "../utils/format";

interface Props {
  account: AccountView;
  now: number;
  onRelogin: () => void;
  /** null 恢复按月推算 */
  onSetRenewal: (at: number | null) => void;
}

function remain(d: number): string {
  return d < 0 ? "已过期" : `剩 ${d} 天`;
}

/** 账号卡片的到期信息：登录有效期 + 订阅续费/到期日 */
export function ExpiryRows({ account: a, now, onRelogin, onSetRenewal }: Props) {
  const loginDays = a.refreshExpiresAt === null ? null : daysLeft(a.refreshExpiresAt, now);
  const loginWarn = a.needsRelogin || (loginDays !== null && loginDays < RELOGIN_WARN_DAYS);
  const renewal = a.renewal;
  const manual = renewal?.source === "manual";

  return (
    <div className="expiry">
      <div className="expiry__row">
        <span className="expiry__label">登录</span>
        <Typography.Text className="expiry__value" type={loginWarn ? "danger" : "secondary"}>
          {a.refreshExpiresAt === null || loginDays === null ? (
            <Tooltip title="通过界面登录添加的账号，在下次刷新令牌后才会显示">有效期未知</Tooltip>
          ) : (
            `${formatDate(a.refreshExpiresAt)} 到期 · ${remain(loginDays)}`
          )}
        </Typography.Text>
        {loginWarn && (
          <Button size="small" type="link" icon={<LoginOutlined />} onClick={onRelogin}>
            重新登录
          </Button>
        )}
      </div>

      <div className="expiry__row">
        <span className="expiry__label">订阅</span>
        <Typography.Text className="expiry__value" type="secondary">
          {renewal
            ? `${manual ? "" : "预计 "}${formatDate(renewal.at)} ${manual ? "到期" : "续费"} · ${remain(daysLeft(renewal.at, now))}`
            : "未知"}
        </Typography.Text>
        <Tooltip title="手动设置实际到期日（如已取消续订）">
          <DatePicker
            size="small"
            variant="borderless"
            className="expiry__picker"
            placeholder="设置到期日"
            value={manual && renewal ? dayjs(renewal.at) : null}
            allowClear={false}
            onChange={(d) => d && onSetRenewal(d.endOf("day").valueOf())}
          />
        </Tooltip>
        {manual && (
          <Tooltip title="恢复按订阅开始日按月推算">
            <Button
              size="small"
              type="text"
              icon={<UndoOutlined />}
              onClick={() => onSetRenewal(null)}
            />
          </Tooltip>
        )}
      </div>
    </div>
  );
}
