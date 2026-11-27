//! 标签（`/tag/gettag`）。

use serde_json::Value;

use super::value;
use crate::error::Result;
use crate::params::Params;
use crate::transport::Transport;

pub struct TagApi<'a> {
    pub(crate) t: &'a Transport,
}

impl TagApi<'_> {
    /// 按标签名取各应用的关联数据；`nums` 为每个应用返回的条数（格式 `appid=数量`）。
    pub async fn get(&self, app_id: i64, tag_name: &str, nums: &[(i64, i64)]) -> Result<Value> {
        let nums: Vec<(String, String)> = nums
            .iter()
            .map(|(app, n)| (app.to_string(), n.to_string()))
            .collect();
        let params = Params::new()
            .set("appid", app_id)
            .set("tagname", tag_name)
            .map("nums", &nums);
        value(self.t, "/tag/gettag", params).await
    }
}
