# 自动更新（tauri-plugin-updater）配置指南

应用内「关于 → Check for Updates」走 Tauri 2 updater 插件：
`tauri.conf.json` 的 `plugins.updater.endpoints` 指向 GitHub Release 的
`latest.json`，客户端校验 minisign 签名后下载安装并重启。

## 一次性配置（维护者）

### 1. 生成签名密钥对

```bash
cd frontend
npm run tauri signer generate -w ~/.tauri/rustfox-updater.key
```

- 私钥：`~/.tauri/rustfox-updater.key`（**丢失后已发布用户将无法静默升级，务必备份**）
- 公钥：`~/.tauri/rustfox-updater.key.pub`

### 2. 公钥写入应用配置

把公钥内容替换 `frontend/src-tauri/tauri.conf.json` 里的占位符：

```json
"plugins": { "updater": { "pubkey": "REPLACE_WITH_TAURI_SIGNER_PUBKEY" } }
```

### 3. 私钥配置到 GitHub Secrets（仓库 Settings → Secrets → Actions）

| Secret | 值 |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | 私钥文件完整内容 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 生成私钥时设置的密码（未设密码则留空/不配） |

## 发布流程（自动）

打 tag（如 `v0.0.3`）触发 `release.yml`：

1. 各平台构建时注入签名私钥，产出 updater 产物（macOS `.app.tar.gz`、
   Linux `.AppImage`、Windows `-setup.exe`）及对应 `.sig` 签名文件；
2. release job 汇总产物，生成 `latest.json`（版本号、各平台下载 URL、签名）
   并附到 GitHub Release；
3. 客户端「检查更新」拉取 `releases/latest/download/latest.json` 对比版本。

> 注意：**未配置 `TAURI_SIGNING_PRIVATE_KEY` secret 前不要打 tag 发版**，
> 构建签名产物时会失败。

## 版本号

发版前同步三处版本：`frontend/package.json`、`frontend/src-tauri/tauri.conf.json`、
`frontend/src-tauri/Cargo.toml`（`rustfox` 包）。updater 以 `tauri.conf.json` 的
`version` 为当前版本，与 `latest.json` 里的 `version` 比较（semver 语义）。
