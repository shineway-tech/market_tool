# GitHub Actions 桌面端发布

这个项目使用 `.github/workflows/release-desktop.yml` 构建桌面端发布包。

## 需要配置的 GitHub Secrets

进入 GitHub 仓库：

`Settings -> Secrets and variables -> Actions -> New repository secret`

添加：

- `TAURI_SIGNING_PRIVATE_KEY`：Tauri updater 私钥
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：Tauri updater 私钥密码，如果生成私钥时没有密码，可以留空

私钥可以用下面命令生成：

```bash
npx tauri signer generate
```

生成后：

- 公钥填到 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`
- 私钥填到 GitHub Secret `TAURI_SIGNING_PRIVATE_KEY`
- 密码填到 GitHub Secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## 发布方式

先确认版本号已经更新：

- `package.json`
- `package-lock.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

然后推送 tag：

```bash
git tag v1.0.0
git push origin v1.0.0
```

Actions 会构建：

- macOS Intel
- macOS Apple Silicon
- Windows x64

构建完成后会创建一个 GitHub Release 草稿，进入 GitHub Release 页面确认并发布。

## 和自建更新服务对接

当前客户端的更新检查地址是：

`https://market-api.honeykid.cn/v1/desktop-updates/{{target}}/{{arch}}/{{current_version}}`

GitHub Release 产物出来后，需要把对应平台的下载地址和 `.sig` 签名内容同步到后端配置或数据库里。

常见平台 key：

- `darwin-x86_64`
- `darwin-aarch64`
- `windows-x86_64`

如果后端继续使用本地文件下载，则把 Release 里的 updater 包下载到：

`server/marketing-master-api/public/desktop-updates`

并更新生产配置中的 `desktop_update.platforms`。
