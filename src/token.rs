//! 应用 Token 缓存。互斥锁覆盖整个换新过程，并发请求共用同一次签发。

use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::sync::Mutex;

use crate::error::Result;
use crate::sign::now_secs;

/// 到期前提前换新的秒数。
const REFRESH_MARGIN_SECS: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenInfo {
    pub token: String,
    /// 绝对 Unix 秒时间戳。
    pub expires_in: i64,
}

impl TokenInfo {
    fn is_fresh(&self, now: i64) -> bool {
        self.expires_in > now + REFRESH_MARGIN_SECS
    }
}

#[derive(Debug, Default)]
pub(crate) struct TokenCache {
    slot: Mutex<Option<TokenInfo>>,
}

impl TokenCache {
    /// 返回有效 Token；缓存缺失或将到期时调用 `fetch` 签发。
    pub async fn get<F, Fut>(&self, fetch: F) -> Result<String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<TokenInfo>>,
    {
        let mut slot = self.slot.lock().await;
        if let Some(info) = slot.as_ref()
            && info.is_fresh(now_secs())
        {
            return Ok(info.token.clone());
        }
        let info = fetch().await?;
        let token = info.token.clone();
        *slot = Some(info);
        Ok(token)
    }

    pub async fn store(&self, info: TokenInfo) {
        *self.slot.lock().await = Some(info);
    }

    /// 仅当缓存仍是 `stale` 时清除，避免覆盖其它请求刚换到的新 Token。
    pub async fn invalidate(&self, stale: &str) {
        let mut slot = self.slot.lock().await;
        if slot.as_ref().is_some_and(|info| info.token == stale) {
            *slot = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn info(token: &str, ttl: i64) -> TokenInfo {
        TokenInfo {
            token: token.to_string(),
            expires_in: now_secs() + ttl,
        }
    }

    #[tokio::test]
    async fn concurrent_requests_share_one_fetch() {
        let cache = Arc::new(TokenCache::default());
        let calls = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let cache = cache.clone();
            let calls = calls.clone();
            tasks.push(tokio::spawn(async move {
                cache
                    .get(|| async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        Ok(info("T1", 1800))
                    })
                    .await
                    .unwrap()
            }));
        }
        for task in tasks {
            assert_eq!(task.await.unwrap(), "T1");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn near_expiry_token_is_refreshed_and_invalidate_is_scoped() {
        let cache = TokenCache::default();
        cache.store(info("OLD", 30)).await;
        let token = cache.get(|| async { Ok(info("NEW", 1800)) }).await.unwrap();
        assert_eq!(token, "NEW");

        cache.invalidate("OLD").await;
        let kept = cache.get(|| async { Ok(info("X", 1800)) }).await.unwrap();
        assert_eq!(kept, "NEW");

        cache.invalidate("NEW").await;
        let renewed = cache.get(|| async { Ok(info("X", 1800)) }).await.unwrap();
        assert_eq!(renewed, "X");
    }
}
