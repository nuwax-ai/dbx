# Fork 定制说明（nuwax-ai/dbx）

本仓库是官方 [t8y2/dbx](https://github.com/t8y2/dbx) 的 fork，**核心用途是 dbx-web 模式的 Docker 自部署**。
相对官方 main 保留少量刻意差异，本文档是这些差异的权威清单，合并官方更新时对照使用。

> 快速定位所有定制点：前端搜 `UPDATER_ENABLED`、`startup.loading` 和注释 `nuwax`，Rust 侧搜 `nuwax`。
> 最近的差异核验：2026-09-28（插件中心/隧道维护入口删除 + 关于我们改名之后）。

## 差异总览

| # | 定制 | 主要文件 | 实现方式 |
|---|------|---------|---------|
| 1 | 屏蔽工具栏「检查更新」入口 | `apps/desktop/src/components/layout/AppToolbar.vue` | `const UPDATER_ENABLED = false` 门控（**不删官方代码**） |
| 2 | 删除工具栏主题切换按钮 | 同上 + `settingsStore.ts` + `settingsSearch.ts` | 整段删除 |
| 3 | 删除工具栏 GitHub 图标 | 同上 | 整段删除 |
| 4 | 「设置 → 关于我们」裁剪 + 改名「版本信息」 | `apps/desktop/src/components/editor/EditorSettingsDialog.vue` + 11 个 locale | 删除社区链接卡片组；`aboutTab` 全语言改为"版本信息"语义 |
| 5 | 应用更新提醒默认关闭 | `apps/desktop/src/stores/settingsStore.ts` | 默认值 true → false |
| 6 | PG 本地免密登录（unix socket） | `crates/dbx-core/src/local_pg.rs` 等 5 处 | 新增模块 + 启动钩子 |
| 7 | Docker 构建加固 | `deploy/Dockerfile` | pip 镜像重试 + amd64 交叉库 |
| 8 | 启动 loading 文案（「正在启动 DBX…」） | `apps/desktop/src/StartupGate.vue` | label 用 `startup.loading` 替代 `migration.checking`（连续加载方案已上游化） |
| 9 | 删除「插件中心」入口（2026-09-28） | `AppToolbar.vue` + `settingsStore.ts` + `settingsSearch.ts` + `App.vue` | 删按钮/菜单项/设置开关；插件系统本身保留 |
| 10 | 删除「隧道维护」设置 tab（2026-09-28） | `EditorSettingsDialog.vue` + `settingsSearch.ts` | 删导航项 + tab 内容 + 搜索条目 |
| 11 | 删除「更新管理」设置 tab（2026-09-28） | `EditorSettingsDialog.vue` + `settingsSearch.ts` | 删导航项 + tab 区块 + 搜索条目；更新设置字段/逻辑保留 |

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

## 4. 「设置 → 关于我们」裁剪 + 改名「版本信息」

**原因**：About 页的 QQ 群/Discord/微信群/飞书群/GitHub 仓库/官方文档卡片对自部署用户是噪音；
裁剪后只剩支持信息和版本号，"关于我们"名字不再贴切。

**保留**：支持信息卡片（about-support）、`SettingsTransferPanel`（配置导入与导出）、
桌面端 `!isWeb` 的调试日志块（web 模式不可见）。
**删除**：上述六张社区链接卡片（`grid gap-3 sm:grid-cols-3` 整块）及 `AppLogo` 导入。

**改名**（2026-09-28）：`aboutTab` 的 locale 值在全部 11 种语言改为"版本信息"语义
（zh-CN「版本信息」、zh-TW「版本資訊」、en "Version Info" 等；ja 本来就是
「バージョン情報」未动）。设置搜索分类标签复用同一 key 自动跟随。
官方若调整各语言 aboutTab 文案，合并时取 main 后需重套。

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
| `crates/dbx-driver-postgres/src/postgres.rs` | `UnixSocketSafeTls`：socket 连接时跳过 TLS。**2026-09-28 官方驱动拆分后新路径**（原 `dbx-drivers/src/db/postgres.rs`，git rename 自动跟随） |

**注意**：官方 2026-09-24 起 `main.rs` 重构为 `fn main() + serve()` 并引入数据迁移门禁
（migration gate + StartupGate 向导）；旧的 `storage.migrate_from_json()` 调用**不要加回**，
JSON 迁移已归迁移门禁管。

## 7. Docker 构建加固

`deploy/Dockerfile` 两处 fork 修改（不影响上游层缓存）：

- **ziglang pip 安装重试**：国内镜像偶发连接死掉，外层循环换新 TCP 连接重试 5 次，独立层隔离
- **amd64 交叉编译库**：arm64 宿主（OrbStack/mac）交叉 amd64 时补装 fontconfig/freetype 的 amd64 dev 库

## 8. 启动屏文案（「正在启动 DBX…」）

**历史**：2026-09-25 fork 曾自研启动屏连续化方案（provide/inject 跳过 App 重复认证 +
`appLoaded` 覆盖层防 chunk 加载空白），解决免密部署下密码页闪现问题。

**2026-09-28 官方上位替代**：官方在 0.6.22+ 实现了等价方案且更完整——`StartupLoading`
组件（index.html 内联 loading 的 Vue 版）、locale 预载（先加载语言再显示文案）、
auth 结果通过 **`startup-authentication` prop** 传给 App（不再重复请求）、异步组件自带
loading/error 组件 + 预载失败恢复。**合并时 StartupGate.vue 和 App.vue 的认证/加载
部分直接取 main 侧**，fork 的旧实现（provide、appLoaded、authCheckPending）全部删除。

**现存 fork 差异只剩一处**（`StartupGate.vue`，带 `nuwax fork` 注释）：启动 loading 文案
用官方 `startup.loading`（"正在启动 DBX…"，StartupLoading 的默认值 key，稳定性最高）而非
`migration.checking`（"正在检查本地数据安全状态…"，措辞对最终用户偏生硬，且该 key 在迁移
向导内部语义正确需保留原值）。冲突时重套这一处即可。

**附带注意**：AppToolbar 里 `useToast` 是否保留取决于官方剩余用法——2026-09-28 起
官方唯一的 `toast(` 用法是主题按钮（fork 删除项），故 fork 侧连 import 一起删；若官方
后续新增别的 toast 用法则恢复。

## 9. 删除「插件中心」入口（2026-09-28）

**原因**：自部署不使用插件市场，管理入口是多余的视觉噪音。

**删除范围**（仅入口，**插件系统/市场/快捷方式全部保留**——官方在大力发展插件功能，动底层必撞）：

- `AppToolbar.vue`：工具栏文字按钮、溢出菜单项、`open-plugin-center` emit、lucide `PlugZap` 导入；
  按钮组 div 保留（仅当 `showPluginCenterShortcuts` 时渲染，作为插件快捷方式的锚点）
- `App.vue`：AppToolbar 上的 `@open-plugin-center` 绑定
- `settingsStore.ts`：`ToolbarItems.pluginCenter` 字段（类型/默认值/normalize 三处）
- `settingsSearch.ts`：`ToolbarVisibilityItemKey` 与 `TOOLBAR_VISIBILITY_ITEMS` 中的 pluginCenter

**保留的打开路径**：`dbx://` 插件安装深链（`openPluginInstallDeepLink`）仍可打开页面——页面本身
未删，只是没有常规入口；标签栏的插件中心 tab 本就只在页面已打开时渲染（非常驻按钮），无需处理；
更新中心内的入口被 UPDATER_ENABLED 门控挡住，不可达。

**合并注意**：官方对 AppToolbar 菜单/按钮组的重构会与此冲突，按"入口全删、系统保留"重套。

## 10. 删除「隧道维护」设置 tab（2026-09-28）

**原因**：自部署不用 SSH 隧道，管理页多余；连接对话框里的临时隧道配置不受影响。

**删除范围**：

- `EditorSettingsDialog.vue`：`settingsCategoryNav` 中的 tunnels 项 + tab 内容
  （`<section v-else-if="activeSettingsTab === 'tunnels'">` + `TunnelProfileManager` 及其 import）
- `settingsSearch.ts`：tunnels 搜索条目 + `SETTINGS_CATEGORIES` 数组中的 "tunnels"
  （`SettingsCategory` **类型**联合保留 "tunnels" 成员——settingsSearch.spec 的
  `Record<SettingsCategory, string>` categoryLabels 仍需要它，且减少类型手术）

**已知瑕疵**：连接对话框中"由共享隧道档案管理，请在 设置 > 隧道维护 中编辑"提示文案仍指向
已删除的 tab（仅使用共享隧道档案的连接可见，自部署场景碰不到，故意不动 locale）。

**合并注意**：官方若往 tunnels tab 加新设置，冲突时保持整块删除；`TunnelProfileManager` 组件
本身保留（连接对话框的隧道选择仍在用共享档案数据）。

## 11. 删除「更新管理」设置 tab（2026-09-28）

**原因**：dbx-web 容器部署无法自更新（与第 1/5 条同源）；工具栏入口早已门控，
设置页里这个 tab 是更新功能最后一个可见入口。

**删除范围**：

- `EditorSettingsDialog.vue`：`settingsCategoryNav` 的 updates 项 + updates tab 的整个
  `<section>`（应用/组件更新状态卡、自动更新开关、下载源选择、ChangelogPanel）+
  `settingsTabsWithApplyFooter` 中的 "updates"
- `settingsSearch.ts`：6 条 `category: "updates"` 搜索条目 + `SETTINGS_CATEGORIES`
  数组项（`SettingsCategory` 类型成员保留，同 tunnels）

**刻意保留的死代码（减少合并冲突）**：`else if (tab === "updates")` 重置分支、
`editAutoUpdateApp` 等脚本状态、`check-updates` emit、相关 props、settingsStore 的
更新字段与 normalize 逻辑——官方怎么改都不撞。**只有 vue-tsc TS6133 报出的未使用
声明才删**（`ChangelogPanel` 导入、`hasAnyUpdate` computed、
`onUpdateDownloadSourceChange` 函数）。

**行为说明**：组件自动更新（驱动/JDBC/MCP/插件）按已保存设置继续工作（默认全开），
只是没有 UI 入口；已存显式配置的实例不受影响。

**注意**：更新中心弹层、工具栏更新图标本就被 `UPDATER_ENABLED` 门控，与此互补——
更新功能的三个可见入口（工具栏/弹层/设置 tab）至此全部不可达。

## 合并摩擦控制原则（2026-09-28 与用户确认）

1. **只删 UI 入口（模板/导航项/搜索条目），官方 script 逻辑尽量原样保留**——死代码
   留着不影响运行，官方怎么改都不冲突
2. **vue-tsc TS6133 报出的未使用声明/导入必须删**（否则编译不过），这是唯一例外，删
   时保持最小化
3. 官方积极开发的区域（插件、更新）用"入口删除 + 系统保留"，不用门控常量也行——
   入口行数少、冲突可控；UPDATER_ENABLED 式门控适合官方频繁重构内部实现的场景

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
- **启动屏连续化/防密码页闪现**（2026-09-28）：官方 0.6.22+ 用 `StartupLoading` + locale 预载 +
  `startup-authentication` prop 实现了等价方案，fork 的 provide/appLoaded/authCheckPending 全部删除，
  StartupGate.vue 与 App.vue 的认证/加载部分合并时直接取 main 侧（文案差异见第 8 条）
