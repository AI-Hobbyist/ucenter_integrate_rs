//! Discuz! UCenter RESTful API 异步客户端。
//!
//! 实际可用的接口取决于 UCenter 后台为应用授权的路径；未授权时返回
//! [`ApiErrorCode::ApiNotPermitted`]。

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
