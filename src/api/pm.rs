//! 短消息（`upload/control/pm.php`）。列表与详情按 UCenter 原样返回 JSON。

use serde_json::Value;

use super::{scalar, value};
use crate::error::Result;
use crate::params::Params;
use crate::transport::Transport;

/// 收件人：按 UID 或用户名指定。
#[derive(Debug, Clone)]
pub enum PmRecipients {
    Uids(Vec<i64>),
    Usernames(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct SendPm {
    pub from_uid: i64,
    pub to: PmRecipients,
    pub subject: String,
    pub message: String,
    /// 回复的消息 ID。
    pub reply_pmid: Option<i64>,
    /// 消息类型：0 私人消息，1 群聊。
    pub kind: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct PmListQuery {
    pub uid: i64,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// `newpm`、`privatepm` 等过滤条件。
    pub filter: Option<String>,
    /// 摘要长度。
    pub msg_len: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct PmViewQuery {
    pub uid: i64,
    pub pmid: Option<i64>,
    pub touid: Option<i64>,
    pub date_range: Option<i64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// `touid` 是否为会话 ID（plid）。
    pub is_plid: bool,
    pub kind: Option<i64>,
}

pub struct PmApi<'a> {
    pub(crate) t: &'a Transport,
}

impl PmApi<'_> {
    /// 新消息数量；`more` 为 1~4 时返回更详细的统计对象。
    pub async fn check_new(&self, uid: i64, more: i64) -> Result<Value> {
        let params = Params::new().set("uid", uid).set("more", more);
        value(self.t, "/pm/check_newpm", params).await
    }

    /// 发送消息，成功返回消息 ID，非正数为失败码。
    pub async fn send(&self, pm: &SendPm) -> Result<i64> {
        let (msgto, is_username) = match &pm.to {
            PmRecipients::Uids(uids) => (join(uids), false),
            PmRecipients::Usernames(names) => (names.join(","), true),
        };
        let params = Params::new()
            .set("fromuid", pm.from_uid)
            .set("msgto", msgto)
            .set("subject", &pm.subject)
            .set("message", &pm.message)
            .opt("replypmid", pm.reply_pmid)
            .flag("isusername", is_username)
            .opt("type", pm.kind);
        scalar(self.t, "/pm/sendpm", params).await
    }

    pub async fn delete(&self, uid: i64, pmids: &[i64]) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .list("pmids", pmids.iter().copied());
        scalar(self.t, "/pm/delete", params).await
    }

    /// 删除会话；`quit` 为真时退出群聊而不是删除。
    pub async fn delete_chat(&self, uid: i64, plids: &[i64], quit: bool) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .list("plids", plids.iter().copied())
            .set("type", if quit { 2 } else { 1 });
        scalar(self.t, "/pm/deletechat", params).await
    }

    /// 删除与指定用户的全部私人会话。
    pub async fn delete_user(&self, uid: i64, touids: &[i64]) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .list("touids", touids.iter().copied());
        scalar(self.t, "/pm/deleteuser", params).await
    }

    /// 设置已读（status=0）或未读（status=1）。
    pub async fn read_status(
        &self,
        uid: i64,
        uids: &[i64],
        plids: &[i64],
        status: i64,
    ) -> Result<()> {
        let params = Params::new()
            .set("uid", uid)
            .list("uids", uids.iter().copied())
            .list("plids", plids.iter().copied())
            .set("status", status);
        self.t.call("/pm/readstatus", &params).await?;
        Ok(())
    }

    /// 忽略全部新消息提醒。
    pub async fn ignore(&self, uid: i64) -> Result<Value> {
        value(self.t, "/pm/ignore", Params::new().set("uid", uid)).await
    }

    pub async fn list(&self, q: &PmListQuery) -> Result<Value> {
        let params = Params::new()
            .set("uid", q.uid)
            .opt("page", q.page)
            .opt("pagesize", q.page_size)
            .opt("filter", q.filter.as_deref())
            .opt("msglen", q.msg_len);
        value(self.t, "/pm/ls", params).await
    }

    pub async fn view(&self, q: &PmViewQuery) -> Result<Value> {
        let params = Params::new()
            .set("uid", q.uid)
            .opt("pmid", q.pmid)
            .opt("touid", q.touid)
            .opt("daterange", q.date_range)
            .opt("page", q.page)
            .opt("pagesize", q.page_size)
            .flag("isplid", q.is_plid)
            .opt("type", q.kind);
        value(self.t, "/pm/view", params).await
    }

    pub async fn view_num(&self, uid: i64, touid: i64, is_plid: bool) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .set("touid", touid)
            .flag("isplid", is_plid);
        scalar(self.t, "/pm/viewnum", params).await
    }

    pub async fn view_node(&self, uid: i64, kind: i64, pmid: i64) -> Result<Value> {
        let params = Params::new()
            .set("uid", uid)
            .set("type", kind)
            .set("pmid", pmid);
        value(self.t, "/pm/viewnode", params).await
    }

    pub async fn chat_members(&self, uid: i64, plid: i64) -> Result<Value> {
        let params = Params::new().set("uid", uid).set("plid", plid);
        value(self.t, "/pm/chatpmmemberlist", params).await
    }

    pub async fn kick_chat(&self, uid: i64, plid: i64, touid: i64) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .set("plid", plid)
            .set("touid", touid);
        scalar(self.t, "/pm/kickchatpm", params).await
    }

    pub async fn append_chat(&self, uid: i64, plid: i64, touid: i64) -> Result<i64> {
        let params = Params::new()
            .set("uid", uid)
            .set("plid", plid)
            .set("touid", touid);
        scalar(self.t, "/pm/appendchatpm", params).await
    }

    /// 黑名单（逗号分隔的用户名）。
    pub async fn blacklist(&self, uid: i64) -> Result<Value> {
        value(self.t, "/pm/blackls_get", Params::new().set("uid", uid)).await
    }

    /// 整体替换黑名单。
    pub async fn set_blacklist(&self, uid: i64, usernames: &[&str]) -> Result<Value> {
        let params = Params::new()
            .set("uid", uid)
            .set("blackls", usernames.join(","));
        value(self.t, "/pm/blackls_set", params).await
    }

    pub async fn add_blacklist(&self, uid: i64, usernames: &[&str]) -> Result<Value> {
        let params = Params::new()
            .set("uid", uid)
            .list("username", usernames.iter());
        value(self.t, "/pm/blackls_add", params).await
    }

    pub async fn remove_blacklist(&self, uid: i64, usernames: &[&str]) -> Result<Value> {
        let params = Params::new()
            .set("uid", uid)
            .list("username", usernames.iter());
        value(self.t, "/pm/blackls_delete", params).await
    }
}

fn join(ids: &[i64]) -> String {
    ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
}
