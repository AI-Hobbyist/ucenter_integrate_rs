//! 邮件队列（`/mail/add`）。

use super::scalar;
use crate::error::Result;
use crate::params::Params;
use crate::transport::Transport;

#[derive(Debug, Clone, Default)]
pub struct MailRequest {
    pub uids: Vec<i64>,
    pub emails: Vec<String>,
    pub subject: String,
    pub message: String,
    pub charset: Option<String>,
    pub html: bool,
    /// 优先级，数值越大越先发送。
    pub level: i64,
    pub from: Option<String>,
}

pub struct MailApi<'a> {
    pub(crate) t: &'a Transport,
}

impl MailApi<'_> {
    /// 加入发送队列，返回邮件 ID。
    pub async fn add(&self, mail: &MailRequest) -> Result<i64> {
        let uids = mail
            .uids
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let params = Params::new()
            .set("uids", uids)
            .set("emails", mail.emails.join(","))
            .set("subject", &mail.subject)
            .set("message", &mail.message)
            .opt("charset", mail.charset.as_deref())
            .set("htmlon", i64::from(mail.html))
            .set("level", mail.level)
            .opt("frommail", mail.from.as_deref());
        scalar(self.t, "/mail/add", params).await
    }
}
