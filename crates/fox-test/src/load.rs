//! M14：接口压测（并发基准测试）。

use std::sync::Arc;
use std::time::Instant;

use fox_core::model::{HttpMethod, RequestSpec};
use tokio::sync::Semaphore;

use fox_http::client::send_request;

/// 压测配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct LoadConfig {
    /// 并发数（同时进行的请求数）。
    pub concurrency: usize,
    /// 总请求数。
    pub total: usize,
}

/// 压测结果。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LoadResult {
    pub total: usize,
    pub ok: usize,
    pub failed: usize,
    pub total_ms: u64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p99_ms: f64,
    pub rps: f64,
    /// 最多保留 5 条请求错误。
    pub errors: Vec<String>,
}

fn percentile(sorted: &[f64], q: usize) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (sorted.len() * q / 100).min(sorted.len() - 1);
    sorted[idx]
}

/// 压测进度快照（回调参数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct LoadProgress {
    pub done: usize,
    pub total: usize,
    pub ok: usize,
    pub failed: usize,
}

/// 并发压测：`total` 次请求，最多 `concurrency` 个同时进行。
/// `progress` 可选：每完成一次请求回调一次快照（用于进度推送）。
pub async fn run_load(
    method: HttpMethod,
    url: &str,
    spec: &RequestSpec,
    cfg: &LoadConfig,
    progress: Option<&(dyn Fn(LoadProgress) + Send + Sync)>,
) -> LoadResult {
    let concurrency = cfg.concurrency.max(1);
    let total = cfg.total.max(1);
    let sem = Arc::new(Semaphore::new(concurrency));
    let start = Instant::now();
    let mut samples: Vec<f64> = Vec::with_capacity(total);
    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut errors: Vec<String> = Vec::new();

    let mut handles = Vec::with_capacity(total);
    for _ in 0..total {
        let permit = sem.clone().acquire_owned().await;
        let Ok(permit) = permit else { break };
        let m = method;
        let u = url.to_string();
        let s = spec.clone();
        handles.push(tokio::spawn(async move {
            let t = Instant::now();
            let r = send_request(m, &u, &s, Some(30_000)).await;
            drop(permit);
            let d = t.elapsed().as_secs_f64() * 1000.0;
            match r {
                Ok(resp) => (true, Some(resp.status), d, None),
                Err(e) => (false, None, d, Some(e.user_message())),
            }
        }));
    }

    for (done, h) in handles.into_iter().enumerate() {
        match h.await {
            Ok((is_ok, _status, d, err)) => {
                samples.push(d);
                if is_ok {
                    ok += 1;
                } else {
                    failed += 1;
                }
                if let Some(e) = err {
                    if errors.len() < 5 {
                        errors.push(e);
                    }
                }
            }
            Err(_) => failed += 1,
        }
        if let Some(cb) = progress {
            cb(LoadProgress {
                done,
                total,
                ok,
                failed,
            });
        }
    }

    let total_ms = start.elapsed().as_millis() as u64;
    let mut sorted = samples.clone();
    sorted.sort_unstable_by(|a, b| a.total_cmp(b));
    let done = total_ms.max(1);
    LoadResult {
        total: ok + failed,
        ok,
        failed,
        total_ms,
        avg_ms: if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f64>() / samples.len() as f64
        },
        p50_ms: percentile(&sorted, 50),
        p90_ms: percentile(&sorted, 90),
        p99_ms: percentile(&sorted, 99),
        rps: (ok + failed) as f64 * 1000.0 / done as f64,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fox_core::model::KeyValue;
    use tokio::net::TcpListener;

    #[test]
    fn percentile_basic() {
        assert_eq!(percentile(&[], 50), 0.0);
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0], 50), 30.0);
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0], 90), 40.0);
        assert_eq!(percentile(&[5.0], 99), 5.0);
    }

    #[tokio::test]
    async fn run_load_basic() {
        let app = axum::Router::new().route("/ping", axum::routing::get(|| async { "pong" }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let spec = RequestSpec {
            params: vec![KeyValue::new("x", "1")],
            ..Default::default()
        };
        let cfg = LoadConfig {
            concurrency: 4,
            total: 20,
        };
        let result = run_load(
            HttpMethod::GET,
            &format!("http://{addr}/ping"),
            &spec,
            &cfg,
            None,
        )
        .await;
        assert_eq!(result.total, 20, "总请求数应等于配置");
        assert_eq!(result.failed, 0, "本地服务不应失败");
        assert_eq!(result.ok, 20);
        assert!(result.avg_ms >= 0.0);
        assert!(result.rps > 0.0);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn run_load_handles_failures() {
        let addr = "127.0.0.1:9"; // discard 端口，连接必然失败。
        let spec = RequestSpec::default();
        let cfg = LoadConfig {
            concurrency: 2,
            total: 6,
        };
        let result = run_load(
            HttpMethod::GET,
            &format!("http://{addr}/nope"),
            &spec,
            &cfg,
            None,
        )
        .await;
        assert_eq!(result.total, 6);
        assert_eq!(result.ok, 0);
        assert_eq!(result.failed, 6);
        assert!(!result.errors.is_empty());
        assert!(result.errors.len() <= 5);
    }
}
