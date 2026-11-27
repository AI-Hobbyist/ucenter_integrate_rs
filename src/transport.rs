//! HTTP 传输：签名、Token 获取与失效重试、响应分类。

use serde_json::Value;

use crate::config::{UcenterConfig, rest_endpoint};
use crate::error::{ApiErrorCode, Result, UcenterError};
use crate::params::Params;
use crate::response::{Response, as_i64};
use crate::sign::build_headers;
use crate::token::{TokenCache, TokenInfo};

pub(crate) struct Transport {
    endpoint: String,
    app_id: String,
    secret: String,
    http: reqwest::Client,
    tokens: TokenCache,
}

impl Transport {
    pub fn new(config: &UcenterConfig) -> Result<Self> {
        let api_url = config.api_url.trim();
        if !api_url.starts_with("http://") && !api_url.starts_with("https://") {
            return Err(UcenterError::Config(
                "UCenter 接口地址需以 http:// 或 https:// 开头".to_string(),
            ));
        }
        let app_id = config.app_id.trim();
        let secret = config.secret.trim();
        if app_id.is_empty() || secret.is_empty() {
            return Err(UcenterError::Config(
                "请填写 UCenter AppID 与通信密钥".to_string(),
            ));
        }
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| UcenterError::Http(format!("初始化 HTTP 客户端失败: {e}")))?;
        Ok(Self {
            endpoint: rest_endpoint(api_url),
            app_id: app_id.to_string(),
            secret: secret.to_string(),
            http,
            tokens: TokenCache::default(),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// 向 `?/token` 申请新的应用 Token 并写入缓存。
    pub async fn refresh_token(&self) -> Result<TokenInfo> {
        let info = self.request_token().await?;
        self.tokens.store(info.clone()).await;
        Ok(info)
    }

    pub async fn token(&self) -> Result<String> {
        self.tokens.get(|| self.request_token()).await
    }

    /// UCenter 站点根地址（去掉 `/api/index.php`），用于公开页面如 `avatar.php`。
    pub fn site_root(&self) -> &str {
        self.endpoint
            .strip_suffix("/api/index.php")
            .unwrap_or(&self.endpoint)
    }

    /// 不签名的 GET 请求（公开页面），跟随跳转，返回响应头与正文。
    pub async fn get_public(&self, url: &str) -> Result<(reqwest::header::HeaderMap, Vec<u8>)> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| UcenterError::Http(format!("请求 UCenter 失败: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(UcenterError::Http(format!(
                "UCenter 返回 HTTP {}",
                status.as_u16()
            )));
        }
        let headers = resp.headers().clone();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| UcenterError::Http(format!("读取 UCenter 响应失败: {e}")))?;
        Ok((headers, bytes.to_vec()))
    }

    async fn request_token(&self) -> Result<TokenInfo> {
        let headers = build_headers(&self.app_id, &self.secret, None)?;
        let body = self.post("/token", headers, &Params::new()).await?;
        let data = Response::parse(&body)?.into_data("/token")?;
        let token = data
            .get("token")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| UcenterError::Protocol("UCenter 未返回有效 token".to_string()))?
            .to_string();
        let expires_in = data
            .get("expires_in")
            .and_then(as_i64)
            .ok_or_else(|| UcenterError::Protocol("UCenter 未返回 expires_in".to_string()))?;
        Ok(TokenInfo { token, expires_in })
    }

    /// 调用业务动作。遇到 Token 缺失、过期或无效时清除缓存并重试一次。
    pub async fn call(&self, action: &str, params: &Params) -> Result<Response> {
        let action = normalize_action(action);
        let token = self.token().await?;
        match self.call_with_token(&action, params, &token).await {
            Err(err) if err.api_code().is_some_and(ApiErrorCode::is_token_error) => {
                self.tokens.invalidate(&token).await;
                let token = self.token().await?;
                self.call_with_token(&action, params, &token).await
            }
            other => other,
        }
    }

    async fn call_with_token(
        &self,
        action: &str,
        params: &Params,
        token: &str,
    ) -> Result<Response> {
        let headers = build_headers(&self.app_id, &self.secret, Some(token))?;
        let body = self.post(action, headers, params).await?;
        Response::parse(&body)
    }

    async fn post(
        &self,
        action: &str,
        headers: reqwest::header::HeaderMap,
        params: &Params,
    ) -> Result<String> {
        let url = format!("{}?{}", self.endpoint, action);
        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .form(params.pairs())
            .send()
            .await
            .map_err(|e| UcenterError::Http(format!("请求 UCenter 失败: {e}")))?;
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
        Ok(text)
    }
}

/// 动作统一为 `/模块/动作` 形式。
fn normalize_action(action: &str) -> String {
    let trimmed = action.trim().trim_start_matches('?');
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_action;

    #[test]
    fn normalizes_action_path() {
        assert_eq!(normalize_action("user/login"), "/user/login");
        assert_eq!(normalize_action("?/user/login"), "/user/login");
        assert_eq!(normalize_action("/pm/ls"), "/pm/ls");
    }
}
