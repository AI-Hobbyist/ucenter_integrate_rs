//! 积分兑换（`/credit/request`）。读取其它应用积分见 `UserApi::get_credit`。

use serde_json::Value;

use super::value;
use crate::error::Result;
use crate::params::Params;
use crate::transport::Transport;

#[derive(Debug, Clone, Copy)]
pub struct CreditExchange {
    pub uid: i64,
    /// 本应用的积分编号。
    pub from: i64,
    /// 目标应用的积分编号。
    pub to: i64,
    pub to_app_id: i64,
    pub amount: i64,
}

pub struct CreditApi<'a> {
    pub(crate) t: &'a Transport,
}

impl CreditApi<'_> {
    /// 发起积分兑换，返回 UCenter 原始结果。
    pub async fn request(&self, req: &CreditExchange) -> Result<Value> {
        let params = Params::new()
            .set("uid", req.uid)
            .set("from", req.from)
            .set("to", req.to)
            .set("toappid", req.to_app_id)
            .set("amount", req.amount);
        value(self.t, "/credit/request", params).await
    }
}
