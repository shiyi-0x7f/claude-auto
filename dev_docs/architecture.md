# 架构说明

## 总览

```
前端（React）                         Rust（Tauri）
 widget 挂件 ─┐                        commands.rs  ── 薄命令层
 settings 设置 ┴─ invoke / "snapshot" ─▶ service.rs   ── 编排：同步 → 轮询 → 评估 → 切换
                    事件推送                 ├─ policy.rs     纯函数：耗尽判定 / 目标挑选
                                             ├─ anthropic.rs  HTTP：额度 / 刷新 / 换码 / profile
                                             ├─ login.rs      PKCE 授权链接 + 换码
                                             ├─ cli_files.rs  读写 CLI 的 .credentials.json / .claude.json
                                             ├─ vault.rs      账号库 accounts.json
                                             ├─ settings.rs   设置 settings.json
                                             ├─ proxy.rs      代理解析（CLI settings.json / 自定义 / 直连）
                                             ├─ privacy.rs    系统级文本打码
                                             ├─ state.rs      运行时状态 + 快照 DTO
                                             └─ tray.rs       托盘与窗口显隐
```

前端只通过 `get_snapshot` 拉首帧，之后都订阅后端推送的 `snapshot` 事件；所有改动走命令，命令完成后后端推送新快照。

## 令牌所有权（核心约束）

Claude 的 refresh token 是**轮换式**的：用一次就发新的，旧的作废。两个进程各自刷新同一账号会互相把对方踢下线，所以：

- **激活账号**（当前写在 CLI `.credentials.json` 里的）的令牌归 **CLI** 所有：本工具只读不刷新；每次轮询/切换前把 CLI 刷新后的令牌同步回账号库（`service::sync_from_cli`）。
- **非激活账号**的令牌归**本工具**所有：剩余有效期 < 10 分钟时由本工具刷新并落库（`service::ensure_fresh`）。
- 切换时先同步收回旧账号令牌，再写入新账号的 `claudeAiOauth` 和 `oauthAccount`。写入用「临时文件 + rename」的原子写，并保留文件中其他键（如 `mcpOAuth`）。CLI 会检测凭据文件 mtime 变化并重新加载，正在运行的会话也会切过去。

账号识别：优先比 `oauthAccount.accountUuid`，缺失时退回比 refresh token。

## 切换策略（policy.rs）

- 触发：激活账号的 5h 或周窗口已用 ≥ 各自阈值；两者都满时报告周限额（恢复更慢）。
- 选目标：排除任一窗口达阈值、额度未知、查询报错或需重新登录的账号，再按 (5h 已用, 周已用) 升序取最小。
- 已过 `resets_at` 的窗口视为已用 0，被切走的账号到点自动恢复为候选。
- 半自动：生成建议 + 系统通知（同一建议只通知一次），用户在挂件 / 托盘 / 设置页确认；全自动：直接切换并通知。

## 锁约定（service.rs）

- `op`（tokio Mutex）串行化所有改动类操作：轮询、切换、导入、登录入库。网络请求期间持有它。
- `inner`（std Mutex）只做短暂的内存读写，不跨 await，前端读快照不会被网络阻塞。
- `net`：HTTP 客户端与生效代理，代理设置变更时先构建新客户端，成功后才落盘并替换。

## 外部接口

均为 Claude Code CLI 内部使用的非公开接口，**逆向自 CLI 2.1.304，2026-10-20 实测可用**。CLI 升级后若失效，先在新版 CLI 二进制里搜索以下常量核对。

| 用途     | 方法 / 地址                                       | 说明                                                                                                                                                      |
| -------- | ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 额度     | `GET https://api.anthropic.com/api/oauth/usage`   | 头：`Authorization: Bearer <access>`、`anthropic-beta: oauth-2025-04-20`；取 `five_hour` / `seven_day` 的 `utilization`（0~100）与 `resets_at`（RFC3339） |
| 账号信息 | `GET https://api.anthropic.com/api/oauth/profile` | 同上；`account.{uuid,email,…}`、`organization.{uuid,organization_type,rate_limit_tier,…}`                                                                 |
| 令牌     | `POST https://platform.claude.com/v1/oauth/token` | JSON；刷新：`grant_type=refresh_token`；换码：`grant_type=authorization_code` + `code_verifier` + `state` + `redirect_uri`                                |
| 授权页   | `https://claude.com/cai/oauth/authorize`          | PKCE S256；`code=true`；`redirect_uri=https://platform.claude.com/oauth/code/callback`（回调页显示 `code#state` 供手动粘贴）                              |

- `client_id`：`9d1c250a-e61b-44d9-88ed-5944d1962f5e`
- 作用域：`org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload user:plugins`
- 直连在部分网络环境会被拒（403 `Request not allowed`），需走代理。

## 本地文件格式

- CLI `.credentials.json` → `claudeAiOauth`：`accessToken` / `refreshToken` / `expiresAt`（毫秒）/ `scopes` / `subscriptionType` / `rateLimitTier`。账号库保存其**原样副本**，避免 CLI 新增字段被丢弃。
- CLI `.claude.json` → `oauthAccount`：`accountUuid` / `emailAddress` / `organizationUuid` / …。界面登录时由 profile 映射生成（`anthropic::build_account`）。
- 配置目录：设置了 `CLAUDE_CONFIG_DIR` 时两个文件都在该目录下；否则凭据在 `~/.claude/`，`.claude.json` 在用户主目录。

## 到期管理（expiry.rs / alerts.rs）

- **登录有效期**：access token 约 8 小时，自动刷新；refresh token 有效期由令牌响应的 `refresh_token_expires_in` 给出（实测约 4 周），记为 `claudeAiOauth.refreshTokenExpiresAt`（与 CLI 字段名一致）。过期后只能重新登录。每次刷新服务端都会下发新的有效期，正常使用中应持续顺延（尚未观察到实际顺延，待验证）。
- **订阅续费日**：CLI 与 profile 均无续费周期接口（只有 `subscription_created_at`；苹果内购由 App Store 管理）。采用「`Account.subscription_expires_override` 手动值优先，否则按订阅开始日加整月推算」。推算总是从开始日加 n 个月，避免月末日期漂移。
- **提醒**：轮询后 `alerts::check` 收集「登录剩 < 7 天 / 已过期」与「订阅剩 < 3 天」，按（账号, 到期时间）去重后发系统通知；挂件气泡与队伍栏 `!` 同步提示。

## 隐私模式

- 挂件：只显示账号名前两个字（`shortLabel`）。
- 设置页：`Masked` 组件渲染马赛克色块，DOM 中不含真实文字；点击后临时显示 6 秒。
- 托盘提示 / 系统通知：`privacy::mask`，保留前两个字加 `***`。
- 日志不受隐私模式影响（仅本地），但从不记录令牌与代理凭据（`proxy::redact`）。

## 已知限制

- 账号库令牌明文存储（与 CLI 自身做法一致），依赖用户目录权限保护。
- macOS 凭据在钥匙串中，未适配。
- `service.rs` 约 480 行，后续可按「同步/轮询」「切换」「登录与账号管理」拆分。
