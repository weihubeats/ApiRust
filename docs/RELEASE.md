# 版本升级与发版流程

RustFox 发版分两步：把版本号从 `X.Y.Z` 升到 `X.Y.(Z+1)`，再用 `vX.Y.(Z+1)` tag 触发 GitHub Actions 产出各平台安装包与 updater 清单。

## 一、提交流程（开发者本仓库）

### 1. 同步三处版本

版本号必须同时更新（否则 updater 或 CI 会拿错当前版本）：

```bash
# frontend/package.json              → "version": "X.Y.Z"
# frontend/src-tauri/tauri.conf.json → "version": "X.Y.Z"
# frontend/src-tauri/Cargo.toml (rustfox 包) → version = "X.Y.Z"
```

- updater 以 `tauri.conf.json` 的 `version` 作为当前客户端版本
- GitHub Actions 打包时读取同处版本号，写入产物名与 `latest.json`

### 2. 同步 package-lock

```bash
cd frontend && npm install --package-lock-only
```

避免 lockfile 里 `package.json` 版本残留（会导致审计/发布脚本拿到旧值）。

### 3. 本地校验

```bash
cd frontend
npm run lint && npm test && npm run build
```

三项全过才能发版。

### 4. 提交并 push

```bash
git add -A
git commit -m "chore: bump version to X.Y.Z"
git push
```

## 二、发版流程（远程 GitHub Release）

### 1. 打 tag 并推送

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

tag 必须匹配 `v*`（`release.yml` 的触发条件是 `push.tags: ["v*"]`），推送后立即触发 CI。

### 2. 验证 CI 产出

`release.yml` 会在各 OS 上跑 `tauri build`（Linux / macOS ARM / macOS Intel / Windows），汇总产物到 `release` job 生成 `latest.json`，最终创建 **Draft Release**。

- **Draft 必须手动点 Publish**，否则 `releases/latest/download/latest.json` 返回 404，updater 拿不到新版本；
- 产物列表：macOS `.dmg` + `.app.tar.gz(.sig)`、Linux `.AppImage`、Windows `.exe` + `.msi`，外加 `latest.json`。

### 3. 验证产物版本

打开 `https://github.com/{owner}/{repo}/releases/latest/download/latest.json`：

```json
{
  "version": "X.Y.Z",
  "notes": "...",
  "pub_date": "...",
  "platforms": {
    "darwin-aarch64": { "url": "...", "signature": "..." },
    "linux-x86_64":   { "url": "...", "signature": "..." }
  }
}
```

`version` 必须是你发的那个 `X.Y.Z`（不是 `main`）。

### 4. 客户端验证

应用内「关于 → Check for Updates」，应弹出对应版本更新提示。

## 三、踩坑清单

- **版本号必须是合法 semver**。`latest.json` 里 `version` 字段会被 `semver` crate 解析，`"main"` 之类的值会直接报 `unexpected character` 错误（曾发生在 `workflow_dispatch` 手动触发、未传 tag 参数的场景）。
- **tag 必须指向 bump 后的 commit**。先打 tag 再 bump → tag 指老 commit，发版后 tag 与 `main` HEAD 不一致。
- **不要手动 dispatch `workflow_dispatch` 不传 tag**。`GITHUB_REF_NAME` 在 dispatch 时是分支名 `main`，会被误当版本写入 `latest.json`。
- **未配置 `TAURI_SIGNING_PRIVATE_KEY` secret 前不要打 tag**。签名失败会导致各平台 tar.gz / AppImage / exe 缺一或无 `.sig`，`latest.json` 不完整。
- **Draft Release 不发出去 = 发不了版**。CI 只创建 draft，最后一步 Publish 必须由人点（安全网，避免草稿也走签名流量）。

## 四、快速命令速查

```bash
# 一键版本同步（替换 X.Y.Z）
sed -i '' 's/"version": "X\.Y\.Z"/"version": "NEW"/' frontend/package.json frontend/src-tauri/tauri.conf.json
# 手动改 Cargo.toml（sed 易出错）

# 一键校验
npm run lint && npm test && npm run build

# 一键发版
git tag vX.Y.Z && git push origin vX.Y.Z
```