# GitHub Actions 桌面端发布

这个项目使用 `.github/workflows/release-desktop.yml` 构建桌面端发布包。

## 需要配置的 GitHub Secrets

进入 GitHub 仓库：

`Settings -> Secrets and variables -> Actions -> New repository secret`

添加：

- `TAURI_SIGNING_PRIVATE_KEY`：Tauri updater 私钥
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：Tauri updater 私钥密码，如果生成私钥时没有密码，可以留空
- `MARKETING_MASTER_RELAY_API_KEY`：Relay 授权 API Key，用于 GitHub Actions 打包客户端时注入

私钥可以用下面命令生成：

```bash
npx tauri signer generate
```

生成后：

- 公钥填到 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`
- 私钥填到 GitHub Secret `TAURI_SIGNING_PRIVATE_KEY`
- 密码填到 GitHub Secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## macOS 签名和公证

如果 macOS DMG 没有 Developer ID 签名和 Apple notarization，用户打开时会看到“不可靠的开发者”或无法验证开发者的提示。发布给普通用户下载的 DMG 需要配置下面这些 Secrets：

- `APPLE_CERTIFICATE`：Developer ID Application `.p12` 证书的 base64 内容
- `APPLE_CERTIFICATE_PASSWORD`：导出 `.p12` 时设置的密码
- `KEYCHAIN_PASSWORD`：CI 临时 keychain 密码，可自行生成一个强密码
- `APPLE_API_ISSUER`：App Store Connect API Issuer ID
- `APPLE_API_KEY`：App Store Connect API Key ID
- `APPLE_API_KEY_BASE64`：App Store Connect API 私钥 `.p8` 文件的 base64 内容

证书要求：

- Apple Developer Program 付费账号
- 证书类型使用 `Developer ID Application`
- 导出证书时要从 Keychain Access 的 `My Certificates` 中导出为 `.p12`

本地生成 base64：

```bash
openssl base64 -A -in DeveloperIDApplication.p12 -out apple-certificate-base64.txt
openssl base64 -A -in AuthKey_XXXXXXXXXX.p8 -out apple-api-key-base64.txt
```

然后分别把两个 txt 文件内容填到 `APPLE_CERTIFICATE` 和 `APPLE_API_KEY_BASE64`。

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

`backend/marketing-master-api/public/desktop-updates`

并更新生产配置中的 `desktop_update.platforms`。
