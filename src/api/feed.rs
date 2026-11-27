//! 动态（`upload/control/feed.php`）。

use serde_json::Value;

use super::{scalar, value};
use crate::error::Result;
use crate::params::Params;
use crate::transport::Transport;

#[derive(Debug, Clone, Default)]
pub struct FeedItem {
    pub app_id: i64,
    pub icon: String,
    pub uid: i64,
    pub username: String,
    /// 支持 `[B]`、`[I]`、`[U]` 标记。
    pub title_template: String,
    pub title_data: Vec<(String, String)>,
    pub body_template: String,
    pub body_data: Vec<(String, String)>,
    pub body_general: String,
    pub target_ids: String,
    /// 最多 4 张图片：（图片地址，链接）。
    pub images: Vec<(String, String)>,
}

pub struct FeedApi<'a> {
    pub(crate) t: &'a Transport,
}

impl FeedApi<'_> {
    /// 添加动态，返回动态 ID。
    pub async fn add(&self, item: &FeedItem) -> Result<i64> {
        let mut params = Params::new()
            .set("appid", item.app_id)
            .set("icon", &item.icon)
            .set("uid", item.uid)
            .set("username", &item.username)
            .set("title_template", &item.title_template)
            .map("title_data", &item.title_data)
            .set("body_template", &item.body_template)
            .map("body_data", &item.body_data)
            .set("body_general", &item.body_general)
            .set("target_ids", &item.target_ids);
        for (index, (image, link)) in item.images.iter().take(4).enumerate() {
            let n = index + 1;
            params = params
                .set(&format!("image_{n}"), image)
                .set(&format!("image_{n}_link"), link);
        }
        scalar(self.t, "/feed/add", params).await
    }

    pub async fn delete(&self, start: i64, limit: i64) -> Result<Value> {
        let params = Params::new().set("start", start).set("limit", limit);
        value(self.t, "/feed/delete", params).await
    }

    /// 取最新动态；`delete` 为真时取出后删除。
    pub async fn get(&self, limit: i64, delete: bool) -> Result<Value> {
        let params = Params::new().set("limit", limit).flag("delete", delete);
        value(self.t, "/feed/get", params).await
    }
}
