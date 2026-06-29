# 客户端架构重构设计

## 背景

当前项目已经完成了基础的前后端拆分，并且 Tauri 侧也开始拆出 `platforms`、`managed_browser` 等模块。但客户端核心仍然存在几个维护风险：

- Tauri `lib.rs`、`platforms/mod.rs`、`managed_browser/mod.rs` 仍承担过多职责。
- 登录、同步、状态检测、打开创作中心之间的会话边界不够统一。
- 平台字段和平台接口需要进一步集中到对应平台目录中，避免横向分支扩散。
- 本地平台账号数据需要按客户端登录用户硬隔离。
- 未来作品管理会带来分页、缓存、状态和平台差异，当前结构需要提前留出扩展空间。

本次重构目标偏激进：保留现有产品体验和数据兼容，重做客户端内部结构。

## 保留边界

必须保持不变：

- 现有界面风格。
- 现有用户数据兼容。
- 后端 API 兼容。
- 自动更新机制。

允许重写：

- 前端页面组织。
- Tauri commands。
- 平台适配层。
- 登录、同步、状态检测、打开创作中心流程。
- 本地平台账号存储结构。

## 非目标

第一阶段不做：

- 作品管理真实接入。
- 真实平台登录 E2E 自动化测试。
- 后端保存平台账号、cookie 或平台 session。
- 使用 DOM 抓取、页面文本解析、截图 OCR 作为账号数据来源。
- 引入 React、Vue 或大型状态管理库。

## 数据归属

服务端只保存客户端用户相关数据：

- 注册、登录、验证码。
- 修改密码、个人信息。
- 反馈。
- 更新公告和自动更新配置。

客户端本地保存平台相关数据：

- 平台账号。
- 平台 cookie。
- browser profile。
- 账号状态。
- 粉丝数、头像缓存等同步结果。

平台数据不上传服务端。不同客户端用户的本地平台账号必须硬隔离。

## 本地数据结构

建议从 JSON 迁移到 SQLite。

```text
app_data/
  local.db
  browser_profiles/
    {user_id}/{platform_id}/{account_id}/
  avatars/
    {user_id}/{platform_id}/{account_id}.png
```

第一阶段 SQLite 表：

```text
app_migrations
platform_accounts
platform_sessions
account_sync_logs
```

设计原则：

- 所有平台账号表必须包含 `user_id`。
- 所有查询必须带 `user_id`。
- 退出登录时清空内存态，不允许复用上一个用户的平台账号。
- 旧 JSON 数据迁移成功后保留备份，不直接删除。

## Tauri 分层

目标结构：

```text
src-tauri/src/
  commands/
    auth.rs
    channels.rs
    platform_auth.rs
    settings.rs
    updater.rs
    profile.rs
  services/
    auth_service.rs
    channel_service.rs
    platform_service.rs
    update_service.rs
  storage/
    local_db.rs
    account_store.rs
    session_store.rs
    migration.rs
  platforms/
    mod.rs
    traits.rs
    shared/
      cookie.rs
      http.rs
      error.rs
      types.rs
    xiaohongshu/
      mod.rs
      api.rs
      auth.rs
      account.rs
      fields.rs
    douyin/
    bilibili/
    kuaishou/
    wechat_channels/
  browser/
    managed.rs
    cdp.rs
    profile.rs
    system_browser.rs
```

职责边界：

- `commands/*`：只负责 Tauri 命令入参、出参和错误包装。
- `services/*`：负责编排业务流程。
- `storage/*`：只负责本地数据读写、事务和迁移。
- `platforms/*`：只负责平台 API、字段解析、cookie 判断。
- `browser/*`：只负责打开登录页、保存 cookie、管理 browser profile。

## PlatformAdapter 契约

平台能力通过统一 adapter 暴露。

```rust
trait PlatformAdapter {
    fn spec(&self) -> PlatformSpec;

    async fn login_url(&self) -> Result<LoginTarget, PlatformError>;

    async fn probe_session(
        &self,
        session: &PlatformSession,
    ) -> Result<AccountProfile, PlatformError>;

    async fn sync_account(
        &self,
        session: &PlatformSession,
    ) -> Result<AccountProfile, PlatformError>;

    async fn open_creator_home(
        &self,
        session: &PlatformSession,
    ) -> Result<(), PlatformError>;

    async fn list_works(
        &self,
        query: WorksQuery,
    ) -> Result<WorksPage, PlatformError> {
        Err(PlatformError::Unsupported)
    }
}
```

第一阶段只实现账号能力：

- `login_url`
- `probe_session`
- `sync_account`
- `open_creator_home`

作品管理只预留接口。

## PlatformSession 模型

登录、同步、状态检测、打开创作中心必须使用同一套会话模型。

```text
PlatformSession
  id
  user_id
  platform_id
  account_id
  cookie_snapshot
  browser_profile_id
  status
  last_verified_at
  created_at
  updated_at
```

统一流程：

- 添加账号：创建 session，打开登录页，调用平台 API 检测，成功后保存账号和 session。
- 刷新账号：读取 session，调用平台 API，更新账号资料。
- 检查状态：读取 session，静默调用平台 API，更新状态。
- 打开创作中心：读取 session，复用 browser profile/cookie，打开平台页面。

## API-only 规则

账号资料和状态检测必须通过平台 API 完成。

允许浏览器做：

- 打开平台登录页。
- 保存和复用 cookie。
- 管理 browser profile。

不允许浏览器做：

- DOM 抓取。
- 页面文本解析。
- 从页面标题、头像图片、元素 class 猜账号。
- 截图 OCR。
- API 失败后写入假成功数据。

失败策略：

- API 返回登录失效：账号状态改为 `expired`。
- API 网络失败或接口结构变化：账号状态改为 `sync_failed` 或 `unknown`，保留旧资料。
- API 成功但关键字段缺失：不覆盖旧数据，记录字段解析错误。
- 新增账号时：必须拿到稳定账号标识后才保存。

## 平台目录规则

每个平台目录必须包含自己的：

- 登录地址。
- 创作中心地址。
- API endpoint。
- 请求头。
- cookie 判断规则。
- uid、昵称、头像、粉丝数、点赞数字段。
- API 响应解析测试 fixture。

公共层只放通用工具，不放具体平台字段。

## 前端分层

保持原生 TypeScript 和 CSS，不引入大型前端框架。

目标结构：

```text
frontend/src/
  app/
    bootstrap.ts
    router.ts
    shell.ts
    session.ts
  pages/
    channels/
      index.ts
      view.ts
      state.ts
      actions.ts
    releases/
    settings/
    feedback/
    profile/
    password/
    auth/
  shared/
    api/
    tauri/
    ui/
    i18n/
    storage/
    types/
```

规则：

- `router` 统一页面切换。
- `pages/*` 只管理自身 view/state/actions。
- `shared/api` 只封装后端请求。
- `shared/tauri` 只封装 Tauri command 调用。
- `shared/ui` 放复用组件。
- 全局状态只放当前用户、主题、语言、更新状态。

## 迁移策略

从旧 JSON 迁移到 SQLite：

1. 启动后检查 migration 版本。
2. 如果 SQLite 未初始化，创建表。
3. 如果存在旧渠道账号 JSON：
   - 带 `user_id` 的账号迁入对应用户。
   - 不带 `user_id` 的账号在当前用户首次登录后迁入当前用户。
4. 迁移成功后写入 `app_migrations`。
5. 旧文件改名备份，例如 `channels.legacy.json`。
6. 迁移失败时不删除旧文件。

## 测试策略

第一阶段必须添加：

- 平台 API 响应解析测试。
- 本地 JSON 到 SQLite 迁移测试。

不做真实平台登录 E2E。

平台解析测试应覆盖：

- uid。
- nickname。
- avatar。
- followers。
- likes。
- 登录失效。
- 字段缺失。

迁移测试应覆盖：

- 多用户隔离。
- 旧账号迁入当前用户。
- session 迁移。
- 不跨用户串号。

## 验收标准

- 客户端能正常启动、登录、退出登录。
- 渠道管理界面风格保持不变。
- 每个平台能新增多个账号。
- 新增账号必须来自平台 API 成功结果。
- 每个平台能通过平台 API 检测状态。
- 每个平台能通过平台 API 同步账号资料。
- 不同客户端用户本地账号完全隔离。
- 旧本地 JSON 数据能迁移到 SQLite。
- 打开创作中心不影响账号同步逻辑。
- 自动更新机制不被破坏。
- `cargo check`、前端 build、平台解析测试、迁移测试通过。
- 搜索确认没有 relay、OAuth、AiToEarn、DOM 抓取旧路径。
- 平台接口和字段记录在对应平台模块中。
