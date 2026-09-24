# Fork 定制说明（nuwax-ai/dbx）

本仓库是官方 [t8y2/dbx](https://github.com/t8y2/dbx) 的 fork，**核心用途是 dbx-web 模式的 Docker 自部署**。
相对官方 main 保留少量刻意差异，本文档是这些差异的权威清单，合并官方更新时对照使用。

> 快速定位所有定制点：前端搜 `UPDATER_ENABLED` 和注释 `nuwax`，Rust 侧搜 `nuwax`。
> 最近的差异核验：2026-09-24（合并官方 734503f4c 之后）。

## 差异总览

| # | 定制 | 主要文件 | 实现方式 |
|---|------|---------|---------|
| 1 | 屏蔽工具栏「检查更新」入口 | `apps/desktop/src/components/layout/AppToolbar.vue` | `const UPDATER_ENABLED = false` 门控（**不删官方代码**） |
| 2 | 删除工具栏主题切换按钮 | 同上 + `settingsStore.ts` + `settingsSearch.ts` | 整段删除 |
| 3 | 删除工具栏 GitHub 图标 | 同上 | 整段删除 |
| 4 | 「设置 → 关于我们」裁剪 | `apps/desktop/src/components/editor/EditorSettingsDialog.vue` | 删除社区链接卡片组 |
| 5 | 应用更新提醒默认关闭 | `apps/desktop/src/stores/settingsStore.ts` | 默认值 true → false |
| 6 | PG 本地免密登录（unix socket） | `crates/dbx-core/src/local_pg.rs` 等 5 处 | 新增模块 + 启动钩子 |
| 7 | Docker 构建加固 | `deploy/Dockerfile` | pip 镜像重试 + amd64 交叉库 |

---

## 1. 屏蔽工具栏「检查更新」入口

**原因**：dbx-web 以容器部署，镜像内的应用无法自更新，更新入口只会造成困惑。

**实现**：官方更新相关代码（props、`ToolbarUpdateIcon.vue`、`showToolbarUpdateEntry` computed、
`checkUpdates` 设置字段）**全部原样保留**，只用 `AppToolbar.vue` 中的常量拦住入口：

```ts
// The in-app updater misbehaves in this fork's Docker deployment; keep its toolbar entries hidden.
const UPDATER_ENABLED = false;
```

共 3 处门控：computed 声明后、溢出菜单 `if (UPDATER_ENABLED && showToolbarUpdateEntry.value)`、
模板 `<template v-if="UPDATER_ENABLED && showToolbarUpdateEntry">`。

**注意**：官方 `ToolbarUpdateIcon.spec.ts` 曾用源码字符串断言检查这些行（2026-09-24 官方已删掉该类断言）。
若官方恢复此类测试，需同步把断言改成带 `UPDATER_ENABLED &&` 前缀。

## 2 & 3. 删除主题切换按钮、GitHub 图标

**原因**：自部署场景不需要外观主题按钮，也不引导用户去官方 GitHub。

**删除范围**（三个文件联动）：

- `AppToolbar.vue`：lucide 的 `Moon/Sun/SunMoon` 图标导入、`appTheme` 导入、`GithubIcon` 组件、
  `isDark`/`themeMode` props、`set-theme-mode`/`open-github` emits、`cycleThemeMode` 等主题函数、
  模板中两个 Tooltip 按钮、菜单项、主题图标 CSS 动画
- `settingsStore.ts`：`ToolbarItems` 的 `theme`/`github` 字段（类型、默认值、normalize 三处）
- `settingsSearch.ts`：`ToolbarVisibilityItemKey` 类型和 `TOOLBAR_VISIBILITY_ITEMS` 列表中的两项

**保留**：`useToast` 导入（官方插件命令功能在用）；App.vue 中 Welcome 页的 `@open-github`
绑定属于另一组件，不动。官方新增的 `alwaysOnTop` 工具栏项**保留**。

## 4. 「设置 → 关于我们」裁剪

**原因**：About 页的 QQ 群/Discord/微信群/飞书群/GitHub 仓库/官方文档卡片对自部署用户是噪音。

**保留**：支持信息卡片（about-support）、`SettingsTransferPanel`（配置导入与导出）、
桌面端 `!isWeb` 的调试日志块（web 模式不可见）。
**删除**：上述六张社区链接卡片（`grid gap-3 sm:grid-cols-3` 整块）及 `AppLogo` 导入。

## 5. 应用更新提醒默认关闭

**原因**：同第 1 条，容器内无法自更新，默认提醒无意义。

`settingsStore.ts` 的 `DEFAULT_EDITOR_SETTINGS` 中三个互为别名的字段默认改为 `false`：

```ts
// nuwax fork: dbx-web deployments cannot self-update; default app update notifications off.
updateNotificationsEnabled: false,
autoDownloadUpdates: false,
autoUpdateApp: false,
```

**组件自动更新不受影响**：`autoUpdateDrivers`/`autoUpdateJdbc`/`autoUpdateMcp`/`autoUpdatePlugins` 保持 `true`。
**注意**：已持久化过显式 `true` 的实例不会被被动关闭，需在 UI 手动关一次。

## 6. PG 本地免密登录（unix socket）

**原因**：容器编排里 PG 与 dbx-web 同机部署时，用 unix socket + trust 认证免密直连，
平台侧重置 PG 密码也不影响 dbx-web。

| 文件 | 内容 |
|------|------|
| `crates/dbx-core/src/local_pg.rs` | 核心模块：`POSTGRES_USER` 环境变量存在时，幂等播种/升级 "Local PostgreSQL" 连接（socket host + 空密码，凭据不落盘） |
| `crates/dbx-core/src/lib.rs` | `pub mod local_pg;`（跟在 `pub mod db;` 后） |
| `crates/dbx-web/src/main.rs` | 启动钩子：`Storage::open_unmigrated(...).with_secret_key_policy(ManagedDataDir)` 之后调用（钩子只写空凭据，pre-migration 写入安全） |
| `crates/dbx-types/src/models/connection.rs` | `postgres_socket_url`：PG 连接串 host 以 `/` 开头时走 socket |
| `crates/dbx-drivers/src/db/postgres.rs` | `UnixSocketSafeTls`：socket 连接时跳过 TLS |

**注意**：官方 2026-09-24 起 `main.rs` 重构为 `fn main() + serve()` 并引入数据迁移门禁
（migration gate + StartupGate 向导）；旧的 `storage.migrate_from_json()` 调用**不要加回**，
JSON 迁移已归迁移门禁管。

## 7. Docker 构建加固

`deploy/Dockerfile` 两处 fork 修改（不影响上游层缓存）：

- **ziglang pip 安装重试**：国内镜像偶发连接死掉，外层循环换新 TCP 连接重试 5 次，独立层隔离
- **amd64 交叉编译库**：arm64 宿主（OrbStack/mac）交叉 amd64 时补装 fontconfig/freetype 的 amd64 dev 库

---

## 测试适配点（官方断言 vs fork 删减）

官方测试会断言被我们删掉的内容，以下文件在合并后需要人工核对/适配：

| 文件 | 适配 |
|------|------|
| `apps/desktop/src/lib/settings/__tests__/settingsSearch.spec.ts` | webKeys 断言改为 `not.toContain("theme")` / `not.toContain("github")` |
| `apps/desktop/src/components/layout/__tests__/AlwaysOnTopVisibilitySync.spec.ts` | 无关字段哨兵从 `toolbarItems.github` 换成 `toolbarItems.checkUpdates` |
| `apps/desktop/src/stores/__tests__/settingsStore.spec.ts` | 更新默认值断言为 `false` |
| `packages/app-tests/settingsStore.test.ts` | 同上（**注意此目录在 apps/desktop 之外，容易漏**） |

## 合并官方 main 的流程

1. `git merge-tree --write-tree --name-only test main` 预判冲突（通常集中在 AppToolbar.vue 等上表文件）
2. 冲突块先取 main 侧还原官方逻辑，再重套本文档 1–5 条差异
3. 验证：
   ```bash
   pnpm install --frozen-lockfile
   pnpm exec vue-tsc --noEmit -p apps/desktop/tsconfig.json   # 必须从仓库根目录跑
   pnpm exec vitest run
   cargo check -p dbx-web -p dbx-core
   cargo nextest run -p dbx-core -p dbx-types -p dbx-drivers local_pg socket
   cargo nextest run -p dbx-web
   ```
4. 全量 vitest 中 `queryStore.*`/`connectionStore.timeout` 等 ~10s 超时用例在本机负载高时会偶发超时，单独重跑即可判定

**已知坑**：官方 main 曾推送过自身测试编译失败的提交（2026-09-24 的 734503f4c 弄坏 dbx-core 测试编译，
官方随后以 d58b3c6a2 修复，本 fork 已 cherry-pick）。遇到 `cargo check -p dbx-core --tests` 编不过，
先在纯 main worktree 上复现确认是否上游问题。

## 已上游化（勿再保留本地版）

- object_cache、query 元数据失效等曾属于 test 分支的功能已通过 PR 进官方 main，合并时直接取 main 侧
- dbx-web 免密码界面：用官方 `DBX_DISABLE_PASSWORD=1` 环境变量，非代码定制
