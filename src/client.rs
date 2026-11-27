//! 客户端入口。按模块分组提供接口，另有通用 `call` 调用任意已授权动作。

use crate::config::UcenterConfig;
use crate::error::Result;
use crate::params::Params;
use crate::response::Response;
use crate::token::TokenInfo;
use crate::transport::Transport;

pub struct UcenterClient {
    transport: Transport,
}

impl UcenterClient {
    pub fn new(config: UcenterConfig) -> Result<Self> {
        Ok(Self {
            transport: Transport::new(&config)?,
        })
    }

    /// RESTful 入口地址（`.../api/index.php`）。
    pub fn endpoint(&self) -> &str {
        self.transport.endpoint()
    }

    /// 重新签发应用 Token，用于连通检测。
    pub async fn check_connection(&self) -> Result<TokenInfo> {
        self.transport.refresh_token().await
    }

    /// 当前有效的应用 Token（必要时自动签发）。
    pub async fn token(&self) -> Result<String> {
        self.transport.token().await
    }

    /// 调用任意动作，例如 `call("/user/get_user", Params::new().set("username", "a"))`。
    /// 动作须已在 UCenter 后台授权给当前应用，否则返回 `ApiErrorCode::ApiNotPermitted`。
    pub async fn call(&self, action: &str, params: Params) -> Result<Response> {
        self.transport.call(action, &params).await
    }

    pub(crate) fn transport(&self) -> &Transport {
        &self.transport
    }
}
