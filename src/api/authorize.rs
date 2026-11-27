//! 授权跳转登录（`/user/authorize`、`/user/check_code`）。
//!
//! 流程：后端取跳转地址 → 浏览器在 UCenter 页面登录 → UCenter 带 `code` 跳回回调地址 →
//! 后端用 `check_code` 兑换 UID。授权码有效期 300 秒，只能兑换一次。

use crate::error::{Result, UcenterError};
use crate::params::Params;
use crate::response::{Response, as_i64, as_string};
use crate::transport::Transport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizeOutcome {
    Success {
        uid: i64,
        username: String,
    },
    /// -1 授权码无效或已过期
    InvalidCode,
    /// -2 授权码对应的用户不存在
    UserNotFound,
    Other(i64),
}

pub struct AuthorizeApi<'a> {
    pub(crate) t: &'a Transport,
}

impl AuthorizeApi<'_> {
    /// 取 UCenter 登录页地址。若后台配置了回调域名白名单，`callback` 的主机须在其中。
    pub async fn url(&self, callback: &str) -> Result<String> {
        let data = self
            .t
            .call("/user/authorize", &Params::new().set("callback", callback))
            .await?
            .into_data("/user/authorize")?;
        data.get("url")
            .and_then(as_string)
            .ok_or_else(|| UcenterError::Protocol("授权响应缺少 url".to_string()))
    }

    /// 兑换回调带回的 `code`。
    pub async fn check_code(&self, code: &str) -> Result<AuthorizeOutcome> {
        let response = self
            .t
            .call("/user/check_code", &Params::new().set("code", code))
            .await?;
        Ok(match response {
            Response::Data(data) => {
                let uid = data
                    .get("uid")
                    .and_then(as_i64)
                    .ok_or_else(|| UcenterError::Protocol("授权结果缺少 uid".to_string()))?;
                let username = data.get("username").and_then(as_string).unwrap_or_default();
                AuthorizeOutcome::Success { uid, username }
            }
            Response::Scalar(-1) => AuthorizeOutcome::InvalidCode,
            Response::Scalar(-2) => AuthorizeOutcome::UserNotFound,
            Response::Scalar(other) => AuthorizeOutcome::Other(other),
            Response::Text(text) => {
                return Err(UcenterError::Protocol(format!(
                    "/user/check_code 返回了文本: {text}"
                )));
            }
        })
    }
}
