use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct UcenterConfig {
    pub api_url: String,
    pub app_id: String,
    pub secret: String,
    pub timeout: Duration,
}

impl UcenterConfig {
    pub fn new(
        api_url: impl Into<String>,
        app_id: impl Into<String>,
        secret: impl Into<String>,
    ) -> Self {
        Self {
            api_url: api_url.into(),
            app_id: app_id.into(),
            secret: secret.into(),
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug)]
pub enum UcenterError {
    Config(String),
    Http(String),
    Protocol(String),
    Api { ret: i64, msg: String },
}

impl fmt::Display for UcenterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(m) => write!(f, "UCenter 配置错误: {m}"),
            Self::Http(m) => write!(f, "UCenter 网络错误: {m}"),
            Self::Protocol(m) => write!(f, "UCenter 协议错误: {m}"),
            Self::Api { ret, msg } => write!(f, "UCenter 错误 ({ret}): {msg}"),
        }
    }
}

impl std::error::Error for UcenterError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub token: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub status: i64,
    pub uid: Option<i64>,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UcUser {
    pub uid: i64,
    pub username: String,
    pub email: String,
}

pub struct UcenterClient {
    config: UcenterConfig,
    endpoint: String,
    http: reqwest::Client,
    cached_token: RwLock<Option<TokenInfo>>,
}

impl UcenterClient {
    pub fn new(config: UcenterConfig) -> Result<Self, UcenterError> {
        let api_url = config.api_url.trim();
        if !api_url.starts_with("http://") && !api_url.starts_with("https://") {
            return Err(UcenterError::Config(
                "UCenter 接口地址需以 http:// 或 https:// 开头".to_string(),
            ));
        }
        if config.app_id.trim().is_empty() || config.secret.trim().is_empty() {
            return Err(UcenterError::Config(
                "请填写 UCenter AppID 与通信密钥".to_string(),
            ));
        }
        let endpoint = rest_endpoint(api_url);
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| UcenterError::Http(format!("初始化 HTTP 客户端失败: {e}")))?;
        Ok(Self {
            config,
            endpoint,
            http,
            cached_token: RwLock::new(None),
        })
    }

    pub async fn fetch_token(&self) -> Result<TokenInfo, UcenterError> {
        let url = format!("{}?/token", self.endpoint);
        let headers = build_headers(self.config.app_id.trim(), self.config.secret.trim(), None)?;
        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| UcenterError::Http(format!("请求 UCenter 失败: {e}")))?;
        let body = parse_json_response(resp).await?;
        let ret = parse_ret(&body).unwrap_or(-1);
        if ret != 0 {
            let msg = body
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("UCenter 鉴权失败")
                .to_string();
            return Err(UcenterError::Api { ret, msg });
        }
        let token = body
            .get("token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| UcenterError::Protocol("UCenter 未返回有效 token".to_string()))?
            .to_string();
        let expires_in = body.get("expires_in").and_then(|v| v.as_i64()).unwrap_or(0);
        let info = TokenInfo { token, expires_in };
        *self.cached_token.write().await = Some(info.clone());
        Ok(info)
    }

    pub async fn token(&self) -> Result<String, UcenterError> {
        let now = now_secs() as i64;
        if let Some(cached) = self.cached_token.read().await.as_ref() {
            if cached.expires_in > now + 60 {
                return Ok(cached.token.clone());
            }
        }
        Ok(self.fetch_token().await?.token)
    }

    pub async fn check_connection(&self) -> Result<TokenInfo, UcenterError> {
        self.fetch_token().await
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ip: Option<&str>,
    ) -> Result<LoginResult, UcenterError> {
        let mut form = vec![
            ("username", username.to_string()),
            ("password", password.to_string()),
        ];
        if let Some(ip) = ip.filter(|s| !s.is_empty()) {
            form.push(("ip", ip.to_string()));
        }
        let body = self.post_action("/user/login", &form).await?;
        let ret = parse_ret(&body).unwrap_or(-1);
        if ret != 0 {
            let msg = body
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("UCenter 登录失败")
                .to_string();
            return Err(UcenterError::Api { ret, msg });
        }
        let data = body
            .get("data")
            .ok_or_else(|| UcenterError::Protocol("UCenter 登录响应缺少 data".to_string()))?;
        let status = data.get("status").and_then(as_i64).unwrap_or(-1);
        let uid = if status > 0 { Some(status) } else { None };
        Ok(LoginResult {
            status,
            uid,
            username: data.get("username").and_then(|v| v.as_str()).map(str::to_string),
            email: data.get("email").and_then(|v| v.as_str()).map(str::to_string),
        })
    }

    pub async fn get_user(
        &self,
        username_or_uid: &str,
        is_uid: bool,
    ) -> Result<Option<UcUser>, UcenterError> {
        let mut form = vec![("username", username_or_uid.to_string())];
        if is_uid {
            form.push(("isuid", "1".to_string()));
        }
        let body = self.post_action("/user/get_user", &form).await?;
        if parse_ret(&body) == Some(0) && body.get("data").is_none() {
            return Ok(None);
        }
        let ret = parse_ret(&body).unwrap_or(-1);
        if ret != 0 {
            let msg = body
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("UCenter 查询用户失败")
                .to_string();
            return Err(UcenterError::Api { ret, msg });
        }
        let data = match body.get("data") {
            Some(v) if v.is_object() => v,
            _ => return Ok(None),
        };
        let uid = data
            .get("uid")
            .and_then(as_i64)
            .ok_or_else(|| UcenterError::Protocol("UCenter 用户缺少 uid".to_string()))?;
        let username = data
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let email = data
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        Ok(Some(UcUser {
            uid,
            username,
            email,
        }))
    }

    pub async fn register(
        &self,
        username: &str,
        password: &str,
        email: &str,
        regip: Option<&str>,
    ) -> Result<i64, UcenterError> {
        let mut form = vec![
            ("username", username.to_string()),
            ("password", password.to_string()),
            ("email", email.to_string()),
        ];
        if let Some(ip) = regip.filter(|s| !s.is_empty()) {
            form.push(("regip", ip.to_string()));
        }
        let body = self.post_action("/user/register", &form).await?;
        parse_ret(&body).ok_or_else(|| UcenterError::Protocol("UCenter 注册响应缺少 ret".to_string()))
    }

    pub async fn check_username(&self, username: &str) -> Result<i64, UcenterError> {
        let form = vec![("username", username.to_string())];
        let body = self.post_action("/user/check_username", &form).await?;
        parse_ret(&body)
            .ok_or_else(|| UcenterError::Protocol("UCenter 用户名检查响应缺少 ret".to_string()))
    }

    pub async fn check_email(&self, email: &str) -> Result<i64, UcenterError> {
        let form = vec![("email", email.to_string())];
        let body = self.post_action("/user/check_email", &form).await?;
        parse_ret(&body)
            .ok_or_else(|| UcenterError::Protocol("UCenter 邮箱检查响应缺少 ret".to_string()))
    }

    async fn post_action(
        &self,
        action: &str,
        form: &[(&str, String)],
    ) -> Result<Value, UcenterError> {
        let tok = self.token().await?;
        let url = format!("{}?{}", self.endpoint, action);
        let headers = build_headers(
            self.config.app_id.trim(),
            self.config.secret.trim(),
            Some(&tok),
        )?;
        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .form(form)
            .send()
            .await
            .map_err(|e| UcenterError::Http(format!("请求 UCenter 失败: {e}")))?;
        parse_json_response(resp).await
    }
}

pub fn rest_endpoint(api_url: &str) -> String {
    let trimmed = api_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/api/index.php") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/api/index.php")
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

pub fn build_headers(
    app_id: &str,
    secret: &str,
    token: Option<&str>,
) -> Result<HeaderMap, UcenterError> {
    let mut nonce_bytes = [0u8; 16];
    getrandom::fill(&mut nonce_bytes)
        .map_err(|e| UcenterError::Config(format!("生成随机数失败: {e}")))?;
    let nonce = hex_lower(&nonce_bytes);
    let t = now_secs().to_string();

    let mut hasher = Sha256::new();
    hasher.update(nonce.as_bytes());
    hasher.update(t.as_bytes());
    hasher.update(secret.as_bytes());
    let digest = hex_lower(&hasher.finalize());
    let sign = BASE64.encode(digest.as_bytes());

    let mut headers = HeaderMap::new();
    let insert = |map: &mut HeaderMap, key: &'static str, val: &str| -> Result<(), UcenterError> {
        let name = HeaderName::from_static(key);
        let value = HeaderValue::from_str(val)
            .map_err(|e| UcenterError::Config(format!("无效请求头 {key}: {e}")))?;
        map.insert(name, value);
        Ok(())
    };
    insert(&mut headers, "appid", app_id)?;
    insert(&mut headers, "nonce", &nonce)?;
    insert(&mut headers, "t", &t)?;
    insert(&mut headers, "sign", &sign)?;
    if let Some(tok) = token.filter(|s| !s.is_empty()) {
        insert(&mut headers, "token", tok)?;
    }
    Ok(headers)
}

async fn parse_json_response(resp: reqwest::Response) -> Result<Value, UcenterError> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| UcenterError::Http(format!("读取 UCenter 响应失败: {e}")))?;
    if !status.is_success() {
        return Err(UcenterError::Http(format!(
            "UCenter 返回 HTTP {}",
            status.as_u16()
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| UcenterError::Protocol(format!("UCenter 返回非 JSON 响应: {e}")))
}

fn parse_ret(body: &Value) -> Option<i64> {
    body.get("ret").and_then(as_i64)
}

fn as_i64(val: &Value) -> Option<i64> {
    val.as_i64()
        .or_else(|| val.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_endpoint_normalizes_trailing_slash() {
        assert_eq!(
            rest_endpoint("https://uc.ai-hobbyist.com/"),
            "https://uc.ai-hobbyist.com/api/index.php"
        );
        assert_eq!(
            rest_endpoint("https://uc.ai-hobbyist.com/api/index.php"),
            "https://uc.ai-hobbyist.com/api/index.php"
        );
    }

    #[test]
    fn build_headers_sets_required_fields() {
        let headers = build_headers("test-app-id", "test-secret", Some("tok123")).unwrap();
        assert_eq!(headers.get("appid").unwrap(), "test-app-id");
        assert_eq!(headers.get("token").unwrap(), "tok123");
        assert_eq!(headers.get("nonce").unwrap().to_str().unwrap().len(), 32);
        assert!(!headers.get("t").unwrap().is_empty());
        assert!(!headers.get("sign").unwrap().is_empty());
    }
}
