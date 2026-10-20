# Claude Auto

Claude Code 多账号凭据管理桌面工具：管理多个 Claude 订阅账号，实时显示 5 小时 / 周限额剩余，额度耗尽时半自动或自动切换 CLI 登录的账号。附带一个 Claude 像素小宠物桌面挂件。

> ⚠️ 依赖 Claude Code CLI 的**非公开内部接口**（额度、OAuth、profile），随 CLI 升级可能失效，详见 [dev_docs/architecture.md](dev_docs/architecture.md#外部接口)。
> 多账号轮换以规避单账号限额可能违反 Anthropic 使用条款，风险自负。

## 功能

- **多凭据管理**：界面内 OAuth 登录（复制授权链接 → 浏览器授权 → 粘贴授权码），或一键导入 CLI 当前登录的账号；不影响 CLI 当前会话。
- **额度监控**：定时查询每个账号的 5 小时窗口与周窗口已用比例和重置时间。
- **自动切换**：当前账号任一窗口达到阈值时，按「排除周额度紧张 → 5h 剩余最多」挑选目标；半自动模式弹通知等确认，全自动模式直接切换。
- **桌面挂件**：像素游戏风状态栏 + Claude 小宠物，心情随额度变化；悬停显示对话气泡，点击宠物可固定。
- **隐私模式**（默认开）：挂件只显示账号名前两个字，设置页显示马赛克（点击临时查看），托盘与通知打码。
- **代理**：所有请求统一走代理，默认读取 CLI `settings.json` 的 `env.HTTPS_PROXY`。

## 技术栈

- 桌面壳：Tauri 2（Rust）—— 凭据读写、额度轮询、切换策略、托盘、通知
- 前端：React 19 + TypeScript + Vite + antd 6
- 包管理：pnpm（前端）、cargo（Rust）
- 测试：vitest（前端）、cargo test（Rust）

## 平台

目前仅支持 **Windows / Linux**：CLI 凭据位于 `<配置目录>/.credentials.json`。macOS 上 CLI 把凭据存在钥匙串里，暂不支持。

## 开发

前置：Node 22+、pnpm 10、Rust stable、已安装 Claude Code CLI。

```bash
pnpm install          # 同时通过 prepare 脚本启用 .githooks（pre-commit 门禁）
pnpm tauri dev        # 启动开发版（挂件 + 托盘；设置窗口从托盘或挂件打开）
```

## 测试与门禁

```bash
pnpm check:all        # 全量门禁：前端 tsc/eslint/prettier/vitest + Rust fmt/clippy/test
pnpm check            # 仅前端
pnpm check:rust       # 仅 Rust
```

pre-commit 钩子与 CI 跑同一套门禁。

## 构建与发布

```bash
pnpm bump 0.2.0       # 统一更新 package.json / tauri.conf.json / Cargo.toml 版本号
git tag v0.2.0 && git push --tags   # 触发 GitHub Actions 构建安装包并发布 Release
```

本地打包：`pnpm tauri build`，产物在 `src-tauri/target/release/bundle/`。

## 数据位置

| 内容                   | 位置                                                                           |
| ---------------------- | ------------------------------------------------------------------------------ |
| 账号库（含令牌，明文） | `%APPDATA%/com.shiyi.claudeauto/accounts.json`                                 |
| 设置                   | `%APPDATA%/com.shiyi.claudeauto/settings.json`                                 |
| 运行日志（正式版）     | `%APPDATA%/com.shiyi.claudeauto/claude-auto.log`                               |
| CLI 凭据（切换时写入） | `$CLAUDE_CONFIG_DIR/.credentials.json`，未设置时 `~/.claude/.credentials.json` |
