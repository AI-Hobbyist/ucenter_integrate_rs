//! 应用、域名与版本信息（`/app/ls`、`/domain/ls`、`/version/check`）。
//!
//! `/app/add` 与 `/app/ucinfo` 需要创始人密码，用于安装流程，未封装；
//! 确需调用时使用 `UcenterClient::call`。

use serde_json::Value;

use super::value;
use crate::error::Result;
use crate::params::Params;
use crate::response::as_string;
use crate::transport::Transport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    /// 程序版本。
    pub file: String,
    /// 数据库版本。
    pub db: String,
}

pub struct SystemApi<'a> {
    pub(crate) t: &'a Transport,
}

impl SystemApi<'_> {
    /// 已接入 UCenter 的应用列表。
    pub async fn apps(&self) -> Result<Value> {
        value(self.t, "/app/ls", Params::new()).await
    }

    /// 应用域名解析列表。
    pub async fn domains(&self) -> Result<Value> {
        value(self.t, "/domain/ls", Params::new()).await
    }

    pub async fn version(&self) -> Result<VersionInfo> {
        let data = self
            .t
            .call("/version/check", &Params::new())
            .await?
            .into_data("/version/check")?;
        let text = |key: &str| data.get(key).and_then(as_string).unwrap_or_default();
        Ok(VersionInfo {
            file: text("file"),
            db: text("db"),
        })
    }
}
