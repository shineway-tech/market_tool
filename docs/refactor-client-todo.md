# 客户端架构重构任务清单

## 阶段 0：准备

- [ ] 冻结当前可运行行为，记录现有渠道管理主流程。
- [ ] 确认当前自动更新、登录、反馈、个人信息功能的 smoke test 步骤。
- [ ] 建立重构分支。
- [ ] 确认本地测试命令：
  - `cargo check`
  - 前端 build
  - Rust 单元测试
  - 前端类型检查

## 阶段 1：建立新边界

- [ ] 新增 `src-tauri/src/commands/` 目录。
- [ ] 新增 `src-tauri/src/services/` 目录。
- [ ] 新增 `src-tauri/src/storage/` 目录。
- [ ] 新增 `src-tauri/src/browser/` 目录。
- [ ] 新增 `src-tauri/src/platforms/traits.rs`。
- [ ] 新增 `src-tauri/src/platforms/shared/`。
- [ ] 保持旧 command 可用，不立即删除旧代码。

## 阶段 2：平台适配层

- [ ] 定义 `PlatformAdapter` trait。
- [ ] 定义 `PlatformSpec`。
- [ ] 定义 `PlatformSession`。
- [ ] 定义 `AccountProfile`。
- [ ] 定义 `PlatformError`。
- [ ] 将小红书迁移到 `platforms/xiaohongshu/`。
- [ ] 将抖音迁移到 `platforms/douyin/`。
- [ ] 将 B 站迁移到 `platforms/bilibili/`。
- [ ] 将快手迁移到 `platforms/kuaishou/`。
- [ ] 将视频号迁移到 `platforms/wechat_channels/`。
- [ ] 每个平台只在自己目录维护 API、headers、cookie、字段 key。
- [ ] 删除平台分支散落在 command/service 层的实现。

## 阶段 3：API-only 账号能力

- [ ] 新增账号必须通过 `PlatformAdapter::probe_session` 成功后保存。
- [ ] 刷新账号必须通过 `PlatformAdapter::sync_account`。
- [ ] 检测状态必须通过 `PlatformAdapter::probe_session`。
- [ ] 打开创作中心只负责打开页面，不负责推断账号状态。
- [ ] 删除 DOM 抓取、页面文本解析、页面元素兜底。
- [ ] API 登录失效时写入 `expired`。
- [ ] API 失败时写入 `sync_failed` 或 `unknown`，并保留旧资料。

## 阶段 4：本地 SQLite 存储

- [ ] 引入本地 SQLite 存储。
- [ ] 创建 `app_migrations` 表。
- [ ] 创建 `platform_accounts` 表。
- [ ] 创建 `platform_sessions` 表。
- [ ] 创建 `account_sync_logs` 表。
- [ ] 所有表写入 `user_id`。
- [ ] 所有账号查询必须带 `user_id`。
- [ ] 实现旧 JSON 读取。
- [ ] 实现旧 JSON 到 SQLite 迁移。
- [ ] 迁移成功后备份旧 JSON。
- [ ] 退出登录后清空当前用户内存态。

## 阶段 5：浏览器会话

- [ ] 将 `managed_browser` 重命名或迁移到 `browser/`。
- [ ] 浏览器模块只负责启动、关闭、profile、cookie。
- [ ] 浏览器模块不依赖渠道账号模型。
- [ ] 登录 session 和主页 session 使用统一 `PlatformSession`。
- [ ] 每个用户、平台、账号使用独立 browser profile。
- [ ] Windows 上主页窗口关闭后可再次打开。
- [ ] Windows 上主页窗口关闭不会卡死主进程。

## 阶段 6：Tauri command/service

- [ ] `commands/channels.rs` 只做参数转换。
- [ ] `services/channel_service.rs` 编排账号增删改查。
- [ ] `services/platform_service.rs` 编排平台登录、同步、状态检测、打开主页。
- [ ] `commands/auth.rs` 保持现有登录、退出登录行为。
- [ ] `commands/updater.rs` 保持自动更新行为。
- [ ] 从 `lib.rs` 移除业务流程。
- [ ] `lib.rs` 只保留模块注册和 Tauri builder。

## 阶段 7：前端结构

- [ ] 新增 `frontend/src/app/router.ts`。
- [ ] 新增 `frontend/src/app/session.ts`。
- [ ] 将 `frontend/src/main.ts` 拆成 bootstrap、router、页面入口。
- [ ] 将渠道管理拆到 `pages/channels/`。
- [ ] 将更新公告拆到 `pages/releases/`。
- [ ] 将系统设置拆到 `pages/settings/`。
- [ ] 将意见反馈拆到 `pages/feedback/`。
- [ ] 将个人信息拆到 `pages/profile/`。
- [ ] 将修改密码拆到 `pages/password/`。
- [ ] 将后端 API 封装到 `shared/api/`。
- [ ] 将 Tauri command 封装到 `shared/tauri/`。
- [ ] 保持现有视觉风格。

## 阶段 8：测试

- [ ] 新增平台 fixture 目录。
- [ ] 添加小红书解析测试。
- [ ] 添加抖音解析测试。
- [ ] 添加 B 站解析测试。
- [ ] 添加快手解析测试。
- [ ] 添加视频号解析测试。
- [ ] 添加登录失效响应测试。
- [ ] 添加字段缺失响应测试。
- [ ] 添加旧 JSON 迁移测试。
- [ ] 添加多用户隔离测试。

## 阶段 9：清理

- [ ] 删除旧 JSON-only 存储逻辑。
- [ ] 删除旧 relay/OAuth/AiToEarn 兼容残留。
- [ ] 删除旧 webview 窗口代码残留。
- [ ] 删除未使用的前端工具和样式。
- [ ] 删除未使用的 Tauri command。
- [ ] 确认 `.gitignore` 覆盖构建产物、日志、临时文件、secret。

## 阶段 10：验收

- [ ] 客户端启动正常。
- [ ] 用户登录正常。
- [ ] 退出登录后回到登录页。
- [ ] 不同用户平台账号不串号。
- [ ] 小红书新增、刷新、状态检测可用。
- [ ] 抖音新增、刷新、状态检测可用。
- [ ] B 站新增、刷新、状态检测可用。
- [ ] 快手新增、刷新、状态检测可用。
- [ ] 视频号新增、刷新、状态检测可用。
- [ ] 打开创作中心不影响同步状态。
- [ ] Windows 主页窗口可关闭、可再次打开。
- [ ] 自动更新仍可检查和下载。
- [ ] 前端 build 通过。
- [ ] `cargo check` 通过。
- [ ] 平台解析测试通过。
- [ ] 迁移测试通过。
- [ ] 搜索确认没有 DOM 抓取账号数据路径。
