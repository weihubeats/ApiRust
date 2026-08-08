# 中国大陆网络下的 Rust 工具链与依赖镜像

在国内安装 Rust 工具链或构建本项目时，官方源（rust-lang.org / static.crates.io）
可能较慢或不可达。下面使用国内镜像（以 rsproxy.cn 为例，清华 TUNA 亦可）。

## 1. rustup（工具链）镜像

```bash
export RUSTUP_DIST_SERVER=https://rsproxy.cn
export RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup
curl --proto '=https' --tlsv1.2 -sSf https://rsproxy.cn/rustup-init.sh | sh
```

验证：

```bash
rustup show            # 应能看到默认工具链版本
cargo --version        # 与 rustup show 一致
```

> 已有工具链但国内访问慢，同样设置上面两个环境变量（写入 `~/.zshrc` 或 `~/.bashrc` 长期生效）。

## 2. crates.io 依赖镜像

编辑 `~/.cargo/config.toml`：

```toml
[source.crates-io]
replace-with = "rsproxy-sparse"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"

[net]
git-fetch-with-cli = true
```

之后 `cargo build` 等命令自动走镜像。

### 备选：清华 TUNA

```toml
[source.crates-io]
replace-with = "tuna"

[source.tuna]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
```

## 3. 验证

```bash
cargo build --release          # 本项目首次全量构建约数分钟
cargo test -p fox-secret       # 小范围快速验证
```

## 4. 离线环境（完全无外网）

若目标机器完全无法联网，可在一台有网的机器上：

```bash
cargo vendor target/vendor
```

将 `target/vendor` 目录与项目一起拷贝过去，并在 `~/.cargo/config.toml` 中配置：

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "target/vendor"
```

> `cargo vendor` 的 vendor 目录需要随项目一起分发；之后构建不再联网。

## 5. 常见问题

| 现象 | 处理 |
| --- | --- |
| 拉取索引时报 TLS/握手错误 | 确认使用 `sparse+https://` 形式；公司网络需代理时参考第 6 节 |
| 下载依赖 502 / 超时 | 换一个镜像（rsproxy ↔ TUNA），或重试（cargo 断点续传） |
| rustup 卡在"downloading rustup-init" | 用 `RUSTUP_DIST_SERVER` 前缀 curl 已包含镜像地址 |

## 6. 代理（可选）

公司内网场景可走 HTTP 代理：

```bash
export https_proxy=http://127.0.0.1:7890
export http_proxy=http://127.0.0.1:7890
```
