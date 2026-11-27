//! 客户端入口。按模块分组提供接口，另有通用 `call` 调用任意已授权动作。

use crate::api::app::SystemApi;
use crate::api::authorize::AuthorizeApi;
use crate::api::avatar::AvatarApi;
use crate::api::credit::CreditApi;
use crate::api::feed::FeedApi;
use crate::api::friend::FriendApi;
use crate::api::mail::MailApi;
use crate::api::pm::PmApi;
use crate::api::tag::TagApi;
use crate::api::user::UserApi;
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

    pub fn user(&self) -> UserApi<'_> {
        UserApi { t: &self.transport }
    }

    pub fn authorize(&self) -> AuthorizeApi<'_> {
        AuthorizeApi { t: &self.transport }
    }

    pub fn avatar(&self) -> AvatarApi<'_> {
        AvatarApi { t: &self.transport }
    }

    pub fn pm(&self) -> PmApi<'_> {
        PmApi { t: &self.transport }
    }

    pub fn friend(&self) -> FriendApi<'_> {
        FriendApi { t: &self.transport }
    }

    pub fn credit(&self) -> CreditApi<'_> {
        CreditApi { t: &self.transport }
    }

    pub fn feed(&self) -> FeedApi<'_> {
        FeedApi { t: &self.transport }
    }

    pub fn mail(&self) -> MailApi<'_> {
        MailApi { t: &self.transport }
    }

    pub fn tag(&self) -> TagApi<'_> {
        TagApi { t: &self.transport }
    }

    /// 应用列表、域名列表与版本信息。
    pub fn system(&self) -> SystemApi<'_> {
        SystemApi { t: &self.transport }
    }
}
