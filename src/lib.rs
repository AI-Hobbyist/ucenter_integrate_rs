//! Discuz! UCenter RESTful API 异步客户端。
//!
//! 接口按 UCenter 控制器分组：`client.user()`、`client.pm()`、`client.avatar()` 等；
//! 未封装的动作可用 [`UcenterClient::call`] 调用。实际可用的接口取决于 UCenter
//! 后台为应用授权的路径，未授权时返回 [`ApiErrorCode::ApiNotPermitted`]。

pub mod api;
mod client;
mod config;
mod error;
mod params;
mod response;
mod sign;
mod token;
mod transport;

pub use client::UcenterClient;
pub use config::{UcenterConfig, rest_endpoint};
pub use error::{ApiErrorCode, Result, UcenterError};
pub use params::Params;
pub use response::{Response, as_i64};
pub use sign::{build_headers, sign};
pub use token::TokenInfo;

pub use api::authorize::AuthorizeOutcome;
pub use api::avatar::{AvatarImage, AvatarImages, AvatarKind, AvatarSize};
pub use api::user::{
    EditOutcome, EditRequest, LoginBy, LoginOutcome, LoginRequest, MergeRequest, Mobile,
    ProtectedUser, RegisterRequest, SecurityQuestion, UcUser, UserCheckError, UserLookup,
};
