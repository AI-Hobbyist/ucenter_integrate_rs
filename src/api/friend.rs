//! 好友（`upload/control/friend.php`）。

use serde_json::Value;

use super::{scalar, value};
use crate::error::Result;
use crate::params::Params;
use crate::transport::Transport;

/// 好友关系方向（`model/friend.php` 的 `direction`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FriendDirection {
    /// 0 我添加的全部好友
    #[default]
    All,
    /// 1 我单向关注的
    Following,
    /// 2 单向关注我的
    Followers,
    /// 3 互为好友
    Mutual,
}

impl FriendDirection {
    fn code(self) -> i64 {
        match self {
            Self::All => 0,
            Self::Following => 1,
            Self::Followers => 2,
            Self::Mutual => 3,
        }
    }
}

pub struct FriendApi<'a> {
    pub(crate) t: &'a Transport,
}

impl FriendApi<'_> {
    pub async fn add(&self, uid: i64, friend_uid: i64, comment: &str) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .set("friendid", friend_uid)
            .set("comment", comment);
        scalar(self.t, "/friend/add", params).await
    }

    pub async fn delete(&self, uid: i64, friend_uids: &[i64]) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .list("friendids", friend_uids.iter().copied());
        scalar(self.t, "/friend/delete", params).await
    }

    pub async fn total(&self, uid: i64, direction: FriendDirection) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .set("direction", direction.code());
        scalar(self.t, "/friend/totalnum", params).await
    }

    pub async fn list(
        &self,
        uid: i64,
        page: i64,
        page_size: i64,
        direction: FriendDirection,
    ) -> Result<Value> {
        let params = Params::new()
            .set("uid", uid)
            .set("page", page)
            .set("pagesize", page_size)
            .set("direction", direction.code());
        value(self.t, "/friend/ls", params).await
    }
}
