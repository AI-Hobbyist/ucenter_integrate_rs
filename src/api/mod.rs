//! 按 UCenter 控制器划分的接口模块。

pub mod app;
pub mod authorize;
pub mod avatar;
pub mod credit;
pub mod feed;
pub mod friend;
pub mod mail;
pub mod pm;
pub mod tag;
pub mod user;

use serde_json::Value;

use crate::error::Result;
use crate::params::Params;
use crate::transport::Transport;

/// 取数字结果码。
pub(crate) async fn scalar(t: &Transport, action: &str, params: Params) -> Result<i64> {
    t.call(action, &params).await?.into_scalar(action)
}

/// 取任意结果并转为 JSON（数组、数字或文本）。
pub(crate) async fn value(t: &Transport, action: &str, params: Params) -> Result<Value> {
    Ok(t.call(action, &params).await?.into_value())
}
