//! 请求签名：`Sign = base64(hex(sha256(Nonce + T + secret)))`。

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, UcenterError};

pub(crate) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// 计算签名值。
pub fn sign(nonce: &str, timestamp: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(nonce.as_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(secret.as_bytes());
    BASE64.encode(hex_lower(&hasher.finalize()).as_bytes())
}

/// 生成一次请求所需的 `Appid`、`Nonce`、`T`、`Sign` 和可选 `Token` 请求头。
pub fn build_headers(app_id: &str, secret: &str, token: Option<&str>) -> Result<HeaderMap> {
    let mut nonce_bytes = [0u8; 16];
    getrandom::fill(&mut nonce_bytes)
        .map_err(|e| UcenterError::Config(format!("生成随机数失败: {e}")))?;
    let nonce = hex_lower(&nonce_bytes);
    let timestamp = now_secs().to_string();
    let signature = sign(&nonce, &timestamp, secret);

    let mut headers = HeaderMap::new();
    insert(&mut headers, "appid", app_id)?;
    insert(&mut headers, "nonce", &nonce)?;
    insert(&mut headers, "t", &timestamp)?;
    insert(&mut headers, "sign", &signature)?;
    if let Some(token) = token.filter(|s| !s.is_empty()) {
        insert(&mut headers, "token", token)?;
    }
    Ok(headers)
}

fn insert(headers: &mut HeaderMap, key: &'static str, value: &str) -> Result<()> {
    let value = HeaderValue::from_str(value)
        .map_err(|e| UcenterError::Config(format!("无效请求头 {key}: {e}")))?;
    headers.insert(HeaderName::from_static(key), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_is_base64_of_lowercase_hex_digest() {
        // 与 PHP 的 base64_encode(hash('sha256', $nonce.$t.$secret)) 一致。
        let expected_hex = hex_lower(&Sha256::digest(b"abc1700000000secret"));
        let decoded = BASE64.decode(sign("abc", "1700000000", "secret")).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), expected_hex);
        assert_eq!(expected_hex.len(), 64);
    }

    #[test]
    fn build_headers_sets_required_fields() {
        let headers = build_headers("test-app-id", "test-secret", Some("tok123")).unwrap();
        assert_eq!(headers.get("appid").unwrap(), "test-app-id");
        assert_eq!(headers.get("token").unwrap(), "tok123");
        assert_eq!(headers.get("nonce").unwrap().len(), 32);
        assert!(!headers.get("t").unwrap().is_empty());
        assert!(!headers.get("sign").unwrap().is_empty());
        assert!(
            build_headers("a", "b", None)
                .unwrap()
                .get("token")
                .is_none()
        );
    }
}
