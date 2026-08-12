//! 基于 `tokio-tungstenite` 的多线程安全 WebSocket 客户端管理器。
//!
//! 特性：
//! - 自定义请求头与子协议（subprotocols）握手；
//! - 心跳 Ping / Pong 检测，失联自动重连（指数退避）；
//! - `tokio::sync::broadcast` 将状态与消息推送给所有订阅者（多窗口 / 多标签 UI 共用）；
//! - 支持发送 Text / Binary / Ping 帧；
//! - 优雅停止（发送 Close 帧后退出）。
//!
//! 使用方式：
//! ```
//! #[tokio::main]
//! async fn main() {
//!     let client = fox_http::ws_client::WsClient::connect(
//!         "ws://127.0.0.1:8080",
//!         Default::default(),
//!         vec![],
//!     )
//!     .await
//!     .unwrap();
//!     let _events = client.subscribe();
//!     client.send_text("hello").await.unwrap();
//!     client.stop().await.unwrap();
//! }
//! ```

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use fox_core::model::WsMessageType;
use fox_core::{ws_error, AppError, Result};
use fox_storage::repository as storage;
use futures::SinkExt;
use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::{ClientRequestBuilder, Message, Utf8Bytes};
use tokio_tungstenite::{connect_async, WebSocketStream};
use uuid::Uuid;

/// 事件广播容量。
const EVENT_CAPACITY: usize = 256;
/// 发送队列容量：有界通道，防止连接卡顿持续发送时内存无限增长；
/// 队列满时新消息直接拒绝（见 [`WsClient::send_message`]）。
const OUTBOX_CAPACITY: usize = 1024;
/// 默认心跳间隔：10 秒。
const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
/// 默认连续未收到 Pong 即判定失联的次数。
const DEFAULT_MAX_MISSED_PONGS: u32 = 3;
/// 默认重连退避初始延迟。
const DEFAULT_BACKOFF_BASE: Duration = Duration::from_millis(500);
/// 默认重连退避最大延迟。
const DEFAULT_BACKOFF_MAX: Duration = Duration::from_secs(30);
/// 持久化待发消息的保留期：超过 24 小时自动清理（过期丢弃）。
const WS_MESSAGE_TTL: chrono::Duration = chrono::Duration::hours(24);

/// 连接状态。所有变化都会通过 [`WsEvent::State`] 推送给订阅者。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsState {
    /// 正在连接 / 重连中。
    Connecting,
    /// 已连接，可收发消息。
    Open,
    /// 已关闭（用户主动停止，或服务端优雅关闭且不再重连）。
    Closed,
    /// 连接异常（未开启自动重连时停留在该状态）。
    Error,
}

/// 一条可发送 / 已接收的 WebSocket 消息。
#[derive(Debug, Clone, PartialEq)]
pub enum WsMessage {
    /// 文本帧。
    Text(String),
    /// 二进制帧。
    Binary(Vec<u8>),
    /// Ping 帧（心跳之外，用户显式发送的探测帧）。
    Ping(Vec<u8>),
}

/// 推送给订阅者的事件（UI 多标签页通过 [`WsClient::subscribe`] 共享同一广播）。
#[derive(Debug, Clone, PartialEq)]
pub enum WsEvent {
    /// 连接状态变化。
    State(WsState),
    /// 收到服务端消息。
    Message(WsMessage),
    /// 连接失败或异常断开（`msg` 为原因）。
    Failed(String),
}

/// 客户端行为选项。
#[derive(Debug, Clone)]
pub struct WsOptions {
    /// 心跳间隔：每经过该时长发送一次 Ping。
    pub heartbeat_interval: Duration,
    /// 连续未收到 Pong 达到该次数即判定连接失活（会触发重连）。
    pub max_missed_pongs: u32,
    /// 重连退避初始延迟（后续按 2 的幂次增长，不超过 `backoff_max`）。
    pub backoff_base: Duration,
    /// 重连退避最大延迟。
    pub backoff_max: Duration,
    /// 断线后是否自动重连。
    pub auto_reconnect: bool,
}

impl Default for WsOptions {
    fn default() -> Self {
        WsOptions {
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            max_missed_pongs: DEFAULT_MAX_MISSED_PONGS,
            backoff_base: DEFAULT_BACKOFF_BASE,
            backoff_max: DEFAULT_BACKOFF_MAX,
            auto_reconnect: true,
        }
    }
}

/// 内部发送队列载荷。
#[derive(Debug, Clone)]
enum WsOutgoing {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
}

impl WsOutgoing {
    fn message_type(&self) -> WsMessageType {
        match self {
            WsOutgoing::Text(_) => WsMessageType::Text,
            WsOutgoing::Binary(_) => WsMessageType::Binary,
            WsOutgoing::Ping(_) => WsMessageType::Ping,
        }
    }

    /// Text 原样；Binary / Ping 编码为 base64（落库格式）。
    fn encoded(&self) -> String {
        match self {
            WsOutgoing::Text(text) => text.clone(),
            WsOutgoing::Binary(data) | WsOutgoing::Ping(data) => {
                base64::engine::general_purpose::STANDARD.encode(data)
            }
        }
    }

    /// 从持久化记录还原（解码失败的消息丢弃）。
    fn from_record(message_type: WsMessageType, payload: &str) -> Option<WsOutgoing> {
        match message_type {
            WsMessageType::Text => Some(WsOutgoing::Text(payload.to_string())),
            WsMessageType::Binary => base64::engine::general_purpose::STANDARD
                .decode(payload)
                .ok()
                .map(WsOutgoing::Binary),
            WsMessageType::Ping => base64::engine::general_purpose::STANDARD
                .decode(payload)
                .ok()
                .map(WsOutgoing::Ping),
        }
    }
}

/// 共享内部状态。
#[derive(Debug)]
struct WsInner {
    target: String,
    headers: HashMap<String, String>,
    subprotocols: Vec<String>,
    options: WsOptions,
    /// 对外广播：状态、消息、错误。
    events: broadcast::Sender<WsEvent>,
    /// 当前连接状态。
    state: watch::Sender<WsState>,
    /// 保持状态通道存活：无接收者时 `Sender::send` 失败且值不更新，
    /// 导致 `state()` 读到陈旧状态（`watch::Receiver` 必须被持有）。
    #[allow(dead_code)]
    state_rx: watch::Receiver<WsState>,
    /// 服务端协商的子协议。
    protocol: watch::Sender<Option<String>>,
    /// 保持子协议通道存活（同上）。
    #[allow(dead_code)]
    protocol_rx: watch::Receiver<Option<String>>,
    /// 停止信号（false→true，只翻转一次）。
    stop: watch::Sender<bool>,
    /// 发送队列（有界缓冲，重连成功后自动补发；满则溢出落库）。
    outbox: mpsc::Sender<WsOutgoing>,
    /// 离线消息持久化存储（可选）：队列满时溢出落库，重连成功后补发。
    store: Option<SqlitePool>,
    /// 后台连接任务句柄。
    task: Mutex<Option<JoinHandle<()>>>,
}

/// 多线程安全的 WebSocket 客户端（克隆共享同一连接会话与事件广播）。
#[derive(Debug, Clone)]
pub struct WsClient {
    inner: Arc<WsInner>,
}

impl WsClient {
    /// 建立连接（默认选项）。立即返回，后台任务负责握手、心跳与重连。
    ///
    /// - `target`：`ws://` 或 `wss://` 地址；
    /// - `headers`：自定义请求头；
    /// - `subprotocols`：请求的子协议列表。
    pub async fn connect(
        target: impl Into<String>,
        headers: HashMap<String, String>,
        subprotocols: Vec<String>,
    ) -> Result<WsClient> {
        Self::connect_with_options(target, headers, subprotocols, WsOptions::default()).await
    }

    /// 带自定义选项建立连接。
    pub async fn connect_with_options(
        target: impl Into<String>,
        headers: HashMap<String, String>,
        subprotocols: Vec<String>,
        options: WsOptions,
    ) -> Result<WsClient> {
        Self::connect_with_store(target, headers, subprotocols, options, None).await
    }

    /// 带自定义选项与持久化存储建立连接。
    ///
    /// `store` 为 SQLite 连接池（`ws_messages` 表）：发送队列满时溢出消息
    /// 自动落库，重连成功后从库中读取并补发；`None` 时保持原有的
    /// 「队列满即丢弃」行为。
    pub async fn connect_with_store(
        target: impl Into<String>,
        headers: HashMap<String, String>,
        subprotocols: Vec<String>,
        options: WsOptions,
        store: Option<SqlitePool>,
    ) -> Result<WsClient> {
        let target = target.into();
        validate_url(&target)?;
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        let (state, state_rx) = watch::channel(WsState::Connecting);
        let (protocol, protocol_rx) = watch::channel(None::<String>);
        let (stop, _) = watch::channel(false);
        let (outbox, outbox_rx) = mpsc::channel(OUTBOX_CAPACITY);
        let inner = Arc::new(WsInner {
            target,
            headers,
            subprotocols,
            options,
            events,
            state,
            state_rx,
            protocol,
            protocol_rx,
            stop,
            outbox,
            store,
            task: Mutex::new(None),
        });
        let task_inner = inner.clone();
        let handle = tokio::spawn(async move {
            ws_loop(task_inner, outbox_rx).await;
        });
        *inner.task.lock().await = Some(handle);
        Ok(WsClient { inner })
    }

    /// 连接目标地址。
    pub fn url(&self) -> &str {
        &self.inner.target
    }

    /// 当前连接状态。
    pub fn state(&self) -> WsState {
        *self.inner.state.borrow()
    }

    /// 服务端协商成功的子协议（尚未协商成功时为 `None`）。
    pub fn subprotocol(&self) -> Option<String> {
        self.inner.protocol.borrow().clone()
    }

    /// 订阅事件流（状态变化 / 收到的消息 / 错误）。多个订阅者互不影响。
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.inner.events.subscribe()
    }

    /// 发送消息。连接未就绪时会进入内部缓冲队列，重连成功后自动补发；
    /// 队列满（背压）时若配置了持久化存储则溢出落库（重连后补发），
    /// 否则返回错误，消息被丢弃而非无限积压。
    pub async fn send_message(&self, message: WsMessage) -> Result<()> {
        let outgoing = match message {
            WsMessage::Text(text) => WsOutgoing::Text(text),
            WsMessage::Binary(data) => WsOutgoing::Binary(data),
            WsMessage::Ping(data) => WsOutgoing::Ping(data),
        };
        match self.inner.outbox.try_send(outgoing) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(outgoing)) => self.persist_overflow(outgoing).await,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(ws_error("WebSocket 客户端已停止，无法发送消息"))
            }
        }
    }

    /// 队列溢出时把消息写入 `ws_messages` 表，等待重连后补发。
    async fn persist_overflow(&self, outgoing: WsOutgoing) -> Result<()> {
        let Some(pool) = &self.inner.store else {
            return Err(ws_error("发送队列已满，消息已丢弃"));
        };
        storage::enqueue_ws_message(
            pool,
            &self.inner.target,
            outgoing.message_type(),
            &outgoing.encoded(),
        )
        .await
        .map(|_| ())
        .map_err(|e| {
            ws_error(format!(
                "发送队列已满，持久化待发消息失败：{}",
                e.user_message()
            ))
        })
    }

    /// 发送文本帧。
    pub async fn send_text(&self, text: impl Into<String>) -> Result<()> {
        self.send_message(WsMessage::Text(text.into())).await
    }

    /// 发送二进制帧。
    pub async fn send_binary(&self, data: Vec<u8>) -> Result<()> {
        self.send_message(WsMessage::Binary(data)).await
    }

    /// 发送 Ping 帧（显式客户端探测）。
    pub async fn send_ping(&self, data: Vec<u8>) -> Result<()> {
        self.send_message(WsMessage::Ping(data)).await
    }

    /// 优雅停止：发送 Close 帧并等待后台任务退出（最多等待 5 秒）。
    pub async fn stop(&self) -> Result<()> {
        let _ = self.inner.stop.send(true);
        let handle = self.inner.task.lock().await.take();
        if let Some(handle) = handle {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }
        let _ = self.inner.state.send(WsState::Closed);
        Ok(())
    }
}

/// 校验目标地址必须是 `ws://` 或 `wss://`。
fn validate_url(target: &str) -> Result<()> {
    let parsed =
        url::Url::parse(target).map_err(|e| ws_error(format!("无效的 WebSocket 地址：{e}")))?;
    match parsed.scheme() {
        "ws" | "wss" => Ok(()),
        scheme => Err(ws_error(format!(
            "不支持的协议：{scheme}（仅支持 ws:// 或 wss://）"
        ))),
    }
}

/// 依据选项计算第 n 次（从 1 开始）重连的退避延迟。
fn backoff_delay(options: &WsOptions, attempt: u32) -> Duration {
    let base_ms = options.backoff_base.as_millis() as u64;
    let max_ms = options.backoff_max.as_millis() as u64;
    let factor = 2u64
        .checked_pow(attempt.saturating_sub(1))
        .unwrap_or(u64::MAX);
    Duration::from_millis(base_ms.saturating_mul(factor).min(max_ms))
}

/// 等待退避延迟；若期间收到停止信号返回 `true`。
async fn wait_stoppable(inner: &WsInner, delay: Duration) -> bool {
    let mut stop = inner.stop.subscribe();
    if *stop.borrow() {
        return true;
    }
    tokio::select! {
        _ = tokio::time::sleep(delay) => false,
        _ = stop.changed() => true,
    }
}

/// 更新状态并广播。
fn set_state(inner: &WsInner, state: WsState) {
    let _ = inner.state.send(state);
    let _ = inner.events.send(WsEvent::State(state));
}

fn emit_failed(inner: &WsInner, reason: impl Into<String>) {
    let _ = inner.events.send(WsEvent::Failed(reason.into()));
}

/// 后台主循环：连接 → 会话 → 断开后按指数退避重连，直到停止或放弃。
async fn ws_loop(inner: Arc<WsInner>, mut outbox_rx: mpsc::Receiver<WsOutgoing>) {
    loop {
        if *inner.stop.borrow() {
            break;
        }
        set_state(&inner, WsState::Connecting);
        // 单次“连接 + 会话”周期，直到本次会话结束。
        let mut attempt: u32 = 1;
        loop {
            if *inner.stop.borrow() {
                break;
            }
            let request = match build_request(&inner) {
                Ok(request) => request,
                Err(e) => {
                    set_state(&inner, WsState::Error);
                    emit_failed(&inner, e.user_message());
                    if !inner.options.auto_reconnect {
                        break;
                    }
                    if wait_stoppable(&inner, backoff_delay(&inner.options, attempt)).await {
                        break;
                    }
                    attempt = attempt.saturating_add(1);
                    continue;
                }
            };
            match connect_async(request).await {
                Err(err) => {
                    set_state(&inner, WsState::Error);
                    emit_failed(&inner, format!("WebSocket 连接失败：{err}"));
                    if !inner.options.auto_reconnect {
                        break;
                    }
                    if wait_stoppable(&inner, backoff_delay(&inner.options, attempt)).await {
                        break;
                    }
                    attempt = attempt.saturating_add(1);
                }
                Ok((ws, response)) => {
                    let _ = inner.protocol.send(extract_subprotocol(&response));
                    set_state(&inner, WsState::Open);
                    let session = run_session(inner.clone(), ws, &mut outbox_rx).await;
                    if *inner.stop.borrow() {
                        break;
                    }
                    match session {
                        Ok(()) => {
                            set_state(&inner, WsState::Closed);
                            if !inner.options.auto_reconnect {
                                break;
                            }
                        }
                        Err(reason) => {
                            set_state(&inner, WsState::Error);
                            emit_failed(&inner, reason);
                            if !inner.options.auto_reconnect {
                                break;
                            }
                        }
                    }
                    // 本次会话结束：按退避延迟等待后重连。
                    if wait_stoppable(&inner, backoff_delay(&inner.options, attempt)).await {
                        break;
                    }
                }
            }
        }
    }
    // 用户主动停止后，保证最终状态为 Closed。
    if *inner.stop.borrow() {
        set_state(&inner, WsState::Closed);
    }
}

/// 构造握手请求（自定义请求头 + 子协议）。
fn build_request(inner: &WsInner) -> Result<ClientRequestBuilder> {
    let uri = inner
        .target
        .parse::<http::Uri>()
        .map_err(|e| ws_error(format!("无效的 WebSocket 地址：{e}")))?;
    let mut builder = ClientRequestBuilder::new(uri);
    for (name, value) in &inner.headers {
        builder = builder.with_header(name.clone(), value.clone());
    }
    for sub in &inner.subprotocols {
        builder = builder.with_sub_protocol(sub.clone());
    }
    Ok(builder)
}

/// 从握手响应中提取服务端协商的子协议。
fn extract_subprotocol(response: &http::Response<Option<Vec<u8>>>) -> Option<String> {
    response
        .headers()
        .get(http::header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// 读取持久化的待发消息（重连成功后补发）。
///
/// 先清理超过 [`WS_MESSAGE_TTL`] 的过期消息，再按时间顺序取回未发送消息。
/// 解码失败（base64 损坏）的消息直接丢弃。
async fn load_persisted(inner: &WsInner) -> VecDeque<(Uuid, WsOutgoing)> {
    let Some(pool) = &inner.store else {
        return VecDeque::new();
    };
    let _ = storage::purge_expired_ws_messages(pool, &inner.target, WS_MESSAGE_TTL).await;
    match storage::list_pending_ws_messages(pool, &inner.target).await {
        Ok(records) => records
            .into_iter()
            .filter_map(|r| {
                WsOutgoing::from_record(r.message_type, &r.payload).map(|msg| (r.id, msg))
            })
            .collect(),
        Err(e) => {
            tracing::warn!("读取待发 WebSocket 消息失败：{}", e.user_message());
            VecDeque::new()
        }
    }
}

/// 取下一条待发送消息：优先补发持久化消息，再消费内存队列。
async fn next_outgoing(
    persisted: &mut VecDeque<(Uuid, WsOutgoing)>,
    outbox_rx: &mut mpsc::Receiver<WsOutgoing>,
) -> Option<(Option<Uuid>, WsOutgoing)> {
    if let Some((id, message)) = persisted.pop_front() {
        return Some((Some(id), message));
    }
    outbox_rx.recv().await.map(|message| (None, message))
}

/// 在位会话：消息双向转发、心跳检测、处理发送队列与停止信号。
/// 返回 `Ok(())` 表示优雅结束（服务端 Close / 用户停止），`Err` 表示异常。
async fn run_session<S>(
    inner: Arc<WsInner>,
    ws: WebSocketStream<S>,
    outbox_rx: &mut mpsc::Receiver<WsOutgoing>,
) -> std::result::Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (mut sink, mut stream) = ws.split();
    let mut stop_rx = inner.stop.subscribe();
    let mut ping = tokio::time::interval(inner.options.heartbeat_interval);
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut missed_pongs: u32 = 0;
    // 重连成功后补发持久化消息（含过期清理）。
    let mut persisted = load_persisted(&inner).await;

    if *stop_rx.borrow() {
        let _ = close_sink(&mut sink).await;
        return Ok(());
    }

    loop {
        tokio::select! {
            // 停止信号：发送 Close 帧并正常退出。
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    let _ = close_sink(&mut sink).await;
                    return Ok(());
                }
            }
            // 出站消息：先补发持久化消息，再消费内存队列（断线期间缓冲，恢复后自动补发）。
            outgoing = next_outgoing(&mut persisted, outbox_rx) => {
                let (persisted_id, message) = match outgoing {
                    Some(pair) => pair,
                    None => {
                        let _ = close_sink(&mut sink).await;
                        return Ok(());
                    }
                };
                let message = match message {
                    WsOutgoing::Text(text) => Message::Text(Utf8Bytes::from(text)),
                    WsOutgoing::Binary(data) => Message::Binary(data.into()),
                    WsOutgoing::Ping(data) => Message::Ping(data.into()),
                };
                if let Err(err) = sink.send(message).await {
                    return Err(format!("消息发送失败：{err}"));
                }
                // 补发成功：删除持久化记录，避免重连后重复发送。
                if let Some(id) = persisted_id {
                    if let Some(pool) = &inner.store {
                        if let Err(e) = storage::delete_ws_messages(pool, &[id]).await {
                            tracing::warn!("删除已发送的 WebSocket 消息失败：{}", e.user_message());
                        }
                    }
                }
            }
            // 入站消息。
            incoming = stream.next() => {
                match incoming {
                    None => return Err("连接已被对端关闭".to_string()),
                    Some(Err(err)) => return Err(format!("连接中断：{err}")),
                    Some(Ok(Message::Text(text))) => {
                        let _ = inner
                            .events
                            .send(WsEvent::Message(WsMessage::Text(text.to_string())));
                    }
                    Some(Ok(Message::Binary(data))) => {
                        let _ = inner
                            .events
                            .send(WsEvent::Message(WsMessage::Binary(data.to_vec())));
                    }
                    // 收到服务端 Ping：自动回 Pong。
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sink.send(Message::Pong(payload)).await;
                    }
                    // 心跳 Pong：重置失联计数。
                    Some(Ok(Message::Pong(_))) => {
                        missed_pongs = 0;
                    }
                    Some(Ok(Message::Close(_))) => {
                        let _ = close_sink(&mut sink).await;
                        return Ok(());
                    }
                    Some(Ok(Message::Frame(_))) => {}
                }
            }
            // 心跳：定时 Ping，连续多次无 Pong 判定连接失效。
            _ = ping.tick() => {
                match sink.send(Message::Ping(bytes::Bytes::new())).await {
                    Ok(()) => {
                        missed_pongs = missed_pongs.saturating_add(1);
                        if missed_pongs >= inner.options.max_missed_pongs {
                            return Err(format!("心跳超时：连续 {missed_pongs} 次未收到 Pong"));
                        }
                    }
                    Err(err) => return Err(format!("心跳发送失败：{err}")),
                }
            }
        }
    }
}

/// 优雅关闭：先发送 Close 帧，再关闭底层连接。
async fn close_sink<S>(
    sink: &mut futures::stream::SplitSink<WebSocketStream<S>, Message>,
) -> std::result::Result<(), AppError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let _ = sink.send(Message::Close(None)).await;
    let _ = sink.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use tokio_tungstenite::tungstenite::http::header::SEC_WEBSOCKET_PROTOCOL;
    use tokio_tungstenite::tungstenite::http::HeaderValue;

    /// 简易回显 WebSocket 服务器：文本/二进制原样回发，Ping 自动回 Pong。
    async fn spawn_echo_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            while let Some(msg) = ws.next().await {
                let Ok(msg) = msg else { break };
                match msg {
                    Message::Text(text) => {
                        let _ = ws.send(Message::Text(text.clone())).await;
                    }
                    Message::Binary(data) => {
                        let _ = ws.send(Message::Binary(data)).await;
                    }
                    Message::Ping(payload) => {
                        let _ = ws.send(Message::Pong(payload)).await;
                    }
                    Message::Close(_) => {
                        let _ = ws.send(Message::Close(None)).await;
                        break;
                    }
                    _ => {}
                }
            }
        });
        format!("ws://{addr}")
    }

    /// 等待一个满足条件的事件（3 秒超时）。
    async fn wait_event<F>(rx: &mut broadcast::Receiver<WsEvent>, predicate: F) -> WsEvent
    where
        F: FnMut(&WsEvent) -> bool,
    {
        let mut predicate = predicate;
        tokio::time::timeout(Duration::from_secs(3), async move {
            loop {
                match rx.recv().await {
                    Ok(event) if predicate(&event) => return event,
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => panic!("事件通道已关闭"),
                }
            }
        })
        .await
        .expect("等待 WebSocket 事件超时")
    }

    #[tokio::test]
    async fn connect_send_receive_round_trip() {
        let url = spawn_echo_server().await;
        let client = WsClient::connect(url, HashMap::new(), vec![])
            .await
            .unwrap();
        let mut events = client.subscribe();

        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;
        assert_eq!(client.state(), WsState::Open);
        assert_eq!(client.subprotocol(), None);

        client.send_text("你好").await.unwrap();
        wait_event(
            &mut events,
            |e| matches!(e, WsEvent::Message(WsMessage::Text(t)) if t == "你好"),
        )
        .await;

        client.send_binary(vec![1, 2, 3]).await.unwrap();
        wait_event(
            &mut events,
            |e| matches!(e, WsEvent::Message(WsMessage::Binary(b)) if b == &vec![1, 2, 3]),
        )
        .await;

        client.stop().await.unwrap();
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Closed)).await;
        assert_eq!(client.state(), WsState::Closed);
    }

    #[tokio::test]
    #[allow(clippy::result_large_err)] // tungstenite::Error 内含 Response，类型由上游定义
    async fn custom_headers_and_subprotocol() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (hdr_tx, hdr_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_hdr_async(
                stream,
                |req: &tokio_tungstenite::tungstenite::http::Request<()>,
                 mut response: tokio_tungstenite::tungstenite::http::Response<()>| {
                    let _ = hdr_tx.send(
                        req.headers()
                            .get("x-api-key")
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_string),
                    );
                    response
                        .headers_mut()
                        .insert(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static("chat"));
                    Ok(response)
                },
            )
            .await
            .unwrap();
            let _ = ws.next().await;
            let _ = ws.close(None).await;
        });

        let mut headers = HashMap::new();
        headers.insert("X-Api-Key".into(), "secret-123".into());
        let client = WsClient::connect(format!("ws://{addr}"), headers, vec!["chat".into()])
            .await
            .unwrap();
        let mut events = client.subscribe();
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;

        assert_eq!(hdr_rx.await.unwrap(), Some("secret-123".to_string()));
        assert_eq!(client.subprotocol(), Some("chat".to_string()));

        client.stop().await.unwrap();
    }

    #[tokio::test]
    async fn auto_reconnect_after_server_drop() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accept_count = Arc::new(AtomicUsize::new(0));
        tokio::spawn({
            let accept_count = accept_count.clone();
            async move {
                // 第一次连接：完成握手后立即断开（模拟服务端崩溃）。
                let (stream, _) = listener.accept().await.unwrap();
                let ws = accept_async(stream).await.unwrap();
                accept_count.fetch_add(1, Ordering::SeqCst);
                drop(ws);
                // 第二次连接：正常回显。
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = accept_async(stream).await.unwrap();
                accept_count.fetch_add(1, Ordering::SeqCst);
                while let Some(msg) = ws.next().await {
                    let Ok(msg) = msg else { break };
                    match msg {
                        Message::Ping(payload) => {
                            let _ = ws.send(Message::Pong(payload)).await;
                        }
                        Message::Close(_) => {
                            let _ = ws.send(Message::Close(None)).await;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });

        let options = WsOptions {
            heartbeat_interval: Duration::from_millis(50),
            max_missed_pongs: 3,
            backoff_base: Duration::from_millis(30),
            backoff_max: Duration::from_millis(150),
            ..Default::default()
        };
        let client =
            WsClient::connect_with_options(format!("ws://{addr}"), HashMap::new(), vec![], options)
                .await
                .unwrap();
        let mut events = client.subscribe();

        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;
        // 服务端断开 → 重连 → 第二次 Open。
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;

        assert!(accept_count.load(Ordering::SeqCst) >= 2);
        client.stop().await.unwrap();
    }

    #[tokio::test]
    async fn graceful_stop_sends_close() {
        let url = spawn_echo_server().await;
        let client = WsClient::connect(url, HashMap::new(), vec![])
            .await
            .unwrap();
        let mut events = client.subscribe();
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;

        client.stop().await.unwrap();
        assert_eq!(client.state(), WsState::Closed);
    }

    #[tokio::test]
    async fn outbox_backpressure_when_queue_full() {
        // 目标不可达：连接一直失败，发送队列无人消费，填满后应返回“队列已满”。
        let client = WsClient::connect("ws://127.0.0.1:1/", HashMap::new(), vec![])
            .await
            .unwrap();
        let mut full_error = None;
        for i in 0..OUTBOX_CAPACITY + 10 {
            match client.send_text(format!("msg{i}")).await {
                Ok(()) => {}
                Err(err) => {
                    full_error = Some(err.user_message().to_string());
                    break;
                }
            }
        }
        client.stop().await.unwrap();
        let message = full_error.expect("发送队列应填满并返回错误");
        assert!(message.contains("发送队列已满"), "意外错误：{message}");
    }

    #[tokio::test]
    async fn overflow_persists_to_store_when_enabled() {
        // 配置持久化存储：队列填满后溢出消息应落库，而不是报错丢弃。
        let pool = fox_storage::db::memory_pool().await.unwrap();
        let target = "ws://127.0.0.1:1/";
        let client = WsClient::connect_with_store(
            target,
            HashMap::new(),
            vec![],
            WsOptions::default(),
            Some(pool.clone()),
        )
        .await
        .unwrap();
        for i in 0..OUTBOX_CAPACITY + 20 {
            client.send_text(format!("msg{i}")).await.unwrap();
        }
        let pending = storage::list_pending_ws_messages(&pool, target)
            .await
            .unwrap();
        assert_eq!(pending.len(), 20, "溢出消息应全部落库");
        assert_eq!(pending[0].message_type, WsMessageType::Text);
        assert_eq!(pending[0].payload, "msg1024");
        client.stop().await.unwrap();
    }

    #[tokio::test]
    async fn persisted_messages_flushed_after_reconnect() {
        // 断线期间积压的消息先落库；重连成功后自动补发并删除记录。
        let pool = fox_storage::db::memory_pool().await.unwrap();
        let received = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn({
            let received = received.clone();
            async move {
                // 第一次连接：完成握手后立即断开（模拟断线）。
                let (stream, _) = listener.accept().await.unwrap();
                let ws = accept_async(stream).await.unwrap();
                drop(ws);
                // 第二次连接：收集消息后正常关闭。
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = accept_async(stream).await.unwrap();
                while let Some(msg) = ws.next().await {
                    let Ok(msg) = msg else { break };
                    match msg {
                        Message::Text(text) => {
                            received.lock().await.push(text.to_string());
                        }
                        Message::Binary(data) => {
                            received.lock().await.push(format!(
                                "bin:{}",
                                base64::engine::general_purpose::STANDARD.encode(data)
                            ));
                        }
                        Message::Ping(payload) => {
                            let _ = ws.send(Message::Pong(payload)).await;
                        }
                        Message::Close(_) => {
                            let _ = ws.send(Message::Close(None)).await;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });
        let target = format!("ws://{addr}");

        let options = WsOptions {
            heartbeat_interval: Duration::from_millis(50),
            max_missed_pongs: 3,
            backoff_base: Duration::from_millis(200),
            backoff_max: Duration::from_millis(500),
            ..Default::default()
        };
        let client = WsClient::connect_with_store(
            target.clone(),
            HashMap::new(),
            vec![],
            options,
            Some(pool.clone()),
        )
        .await
        .unwrap();
        let mut events = client.subscribe();
        // 第一次 Open 后立即被服务端断开。
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;
        // 等待断线事件：此刻连接不可用，消息只能落库。
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Error)).await;

        // 断线期间写入持久化消息（text 原样 / binary base64）。
        storage::enqueue_ws_message(&pool, &target, WsMessageType::Text, "p1")
            .await
            .unwrap();
        storage::enqueue_ws_message(&pool, &target, WsMessageType::Text, "p2")
            .await
            .unwrap();
        storage::enqueue_ws_message(
            &pool,
            &target,
            WsMessageType::Binary,
            &base64::engine::general_purpose::STANDARD.encode([1, 2, 3]),
        )
        .await
        .unwrap();

        // 重连成功 → 补发持久化消息。
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;

        client.send_text("live").await.unwrap();

        // 等服务端收齐 4 条消息（p1、p2、binary、live）。
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            if received.lock().await.len() >= 4 {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "补发消息未收齐");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let got = received.lock().await.clone();
        assert!(got.contains(&"p1".to_string()));
        assert!(got.contains(&"p2".to_string()));
        assert!(got.contains(&"live".to_string()));
        assert!(
            got.contains(&"bin:AQID".to_string()),
            "二进制消息应补发：{got:?}"
        );

        // 补发成功后持久化记录应被删除。
        let pending = storage::list_pending_ws_messages(&pool, &target)
            .await
            .unwrap();
        assert!(pending.is_empty(), "补发后记录应删除：{pending:?}");

        client.stop().await.unwrap();
    }

    #[tokio::test]
    async fn persisted_expired_messages_cleaned_on_reconnect() {
        let pool = fox_storage::db::memory_pool().await.unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            while let Some(msg) = ws.next().await {
                let Ok(msg) = msg else { break };
                match msg {
                    Message::Ping(payload) => {
                        let _ = ws.send(Message::Pong(payload)).await;
                    }
                    Message::Close(_) => {
                        let _ = ws.send(Message::Close(None)).await;
                        break;
                    }
                    _ => {}
                }
            }
        });
        let target = format!("ws://{addr}");
        let record = storage::enqueue_ws_message(&pool, &target, WsMessageType::Text, "stale")
            .await
            .unwrap();
        let old = (chrono::Utc::now() - chrono::Duration::hours(48)).to_rfc3339();
        sqlx::query("UPDATE ws_messages SET created_at = ? WHERE id = ?")
            .bind(old)
            .bind(record.id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        let client = WsClient::connect_with_store(
            target.clone(),
            HashMap::new(),
            vec![],
            WsOptions::default(),
            Some(pool.clone()),
        )
        .await
        .unwrap();
        let mut events = client.subscribe();
        // 连接成功 → 会话启动时清理过期消息。
        wait_event(&mut events, |e| *e == WsEvent::State(WsState::Open)).await;
        let pending = storage::list_pending_ws_messages(&pool, &target)
            .await
            .unwrap();
        assert!(pending.is_empty(), "过期消息应被清理：{pending:?}");
        client.stop().await.unwrap();
    }

    #[tokio::test]
    async fn rejects_non_ws_url() {
        let err = WsClient::connect("http://example.com", HashMap::new(), vec![])
            .await
            .unwrap_err();
        assert!(err.user_message().contains("仅支持 ws:// 或 wss://"));
    }
}
