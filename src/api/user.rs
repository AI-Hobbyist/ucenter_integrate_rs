//! 用户接口（`upload/control/user.php`）。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Result, UcenterError};
use crate::params::Params;
use crate::response::{Response, as_i64, as_string};
use crate::transport::Transport;

/// 注册、预检、改名等接口共用的用户校验结果码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserCheckError {
    /// -1 用户名不合法
    UsernameInvalid,
    /// -2 用户名包含禁止注册的词语
    UsernameBadword,
    /// -3 用户名已存在
    UsernameExists,
    /// -4 邮箱格式有误
    EmailInvalid,
    /// -5 邮箱不允许注册
    EmailRestricted,
    /// -6 邮箱已被注册
    EmailExists,
    /// -7 改名失败
    UsernameChangeFailed,
    /// -9 手机号已存在
    MobileExists,
    /// -10 扩展字段无效
    FieldsInvalid,
    Other(i64),
}

impl UserCheckError {
    pub fn from_code(code: i64) -> Self {
        match code {
            -1 => Self::UsernameInvalid,
            -2 => Self::UsernameBadword,
            -3 => Self::UsernameExists,
            -4 => Self::EmailInvalid,
            -5 => Self::EmailRestricted,
            -6 => Self::EmailExists,
            -7 => Self::UsernameChangeFailed,
            -9 => Self::MobileExists,
            -10 => Self::FieldsInvalid,
            other => Self::Other(other),
        }
    }

    pub fn code(self) -> i64 {
        match self {
            Self::UsernameInvalid => -1,
            Self::UsernameBadword => -2,
            Self::UsernameExists => -3,
            Self::EmailInvalid => -4,
            Self::EmailRestricted => -5,
            Self::EmailExists => -6,
            Self::UsernameChangeFailed => -7,
            Self::MobileExists => -9,
            Self::FieldsInvalid => -10,
            Self::Other(code) => code,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::UsernameInvalid => "用户名不合法",
            Self::UsernameBadword => "用户名包含禁止注册的词语",
            Self::UsernameExists => "用户名已存在",
            Self::EmailInvalid => "邮箱格式有误",
            Self::EmailRestricted => "该邮箱不允许注册",
            Self::EmailExists => "邮箱已被注册",
            Self::UsernameChangeFailed => "修改用户名失败",
            Self::MobileExists => "手机号已被使用",
            Self::FieldsInvalid => "扩展字段无效",
            Self::Other(_) => "UCenter 拒绝了该操作",
        }
    }
}

/// 正数为成功值，非正数映射为校验错误。
fn positive_or_check(code: i64) -> std::result::Result<i64, UserCheckError> {
    if code > 0 {
        Ok(code)
    } else {
        Err(UserCheckError::from_code(code))
    }
}

/// 登录账号的类型，对应 `isuid`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoginBy {
    #[default]
    Username,
    Uid,
    Email,
    /// 账号写作 `区号-号码`。
    Mobile,
}

impl LoginBy {
    fn isuid(self) -> Option<i64> {
        match self {
            Self::Username => None,
            Self::Uid => Some(1),
            Self::Email => Some(2),
            Self::Mobile => Some(4),
        }
    }
}

/// 安全提问。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityQuestion {
    pub question_id: i64,
    pub answer: String,
}

/// 手机号（区号与号码分开）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mobile {
    pub country_code: String,
    pub number: String,
}

#[derive(Debug, Clone, Default)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
    pub by: LoginBy,
    /// 设置后 UCenter 会校验安全提问。
    pub question: Option<SecurityQuestion>,
    /// 客户端 IP。UCenter 仅在提供 IP 时执行登录失败次数限制。
    pub ip: Option<String>,
    /// 为真时不记录失败次数。
    pub nolog: bool,
}

impl LoginRequest {
    pub fn new(account: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            account: account.into(),
            password: password.into(),
            ..Self::default()
        }
    }

    pub fn by(mut self, by: LoginBy) -> Self {
        self.by = by;
        self
    }

    pub fn ip(mut self, ip: impl Into<String>) -> Self {
        self.ip = Some(ip.into());
        self
    }

    pub fn question(mut self, question: SecurityQuestion) -> Self {
        self.question = Some(question);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    Success {
        uid: i64,
        username: String,
        email: String,
    },
    /// -1
    UserNotFound,
    /// -2
    WrongPassword,
    /// -3
    QuestionMismatch,
    /// -4 失败次数过多，暂时禁止登录
    TooManyAttempts,
    Other(i64),
}

/// `get_user` 的查询方式。
#[derive(Debug, Clone, Copy)]
pub enum UserLookup<'a> {
    Username(&'a str),
    Uid(i64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UcUser {
    pub uid: i64,
    pub username: String,
    pub email: String,
    /// UCenter 后台配置的扩展字段。
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
    pub question: Option<SecurityQuestion>,
    pub mobile: Option<Mobile>,
    /// 注册 IP。
    pub ip: Option<String>,
}

impl RegisterRequest {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        email: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            email: email.into(),
            ..Self::default()
        }
    }

    pub fn ip(mut self, ip: impl Into<String>) -> Self {
        self.ip = Some(ip.into());
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct EditRequest {
    pub username: String,
    /// 当前密码；`ignore_old_password` 为真时可省略。
    pub old_password: Option<String>,
    pub new_password: Option<String>,
    /// UCenter 每次都会写入邮箱，修改密码时也应传当前邮箱。
    pub email: Option<String>,
    pub ignore_old_password: bool,
    pub question: Option<SecurityQuestion>,
    pub mobile: Option<Mobile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOutcome {
    /// 受影响行数。
    Updated(i64),
    /// 0：没有数据变化。
    Unchanged,
    /// -1：旧密码错误。
    WrongOldPassword,
    /// -7：没有可修改的内容。
    NothingToUpdate,
    /// -8：受保护的用户，不允许修改。
    Protected,
    /// 邮箱或手机号校验失败。
    Rejected(UserCheckError),
}

impl EditOutcome {
    fn from_code(code: i64) -> Self {
        match code {
            n if n > 0 => Self::Updated(n),
            0 => Self::Unchanged,
            -1 => Self::WrongOldPassword,
            -7 => Self::NothingToUpdate,
            -8 => Self::Protected,
            other => Self::Rejected(UserCheckError::from_code(other)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MergeRequest {
    pub old_username: String,
    pub new_username: String,
    pub uid: i64,
    pub password: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedUser {
    pub uid: i64,
    pub username: String,
}

pub struct UserApi<'a> {
    pub(crate) t: &'a Transport,
}

impl UserApi<'_> {
    pub async fn login(&self, req: &LoginRequest) -> Result<LoginOutcome> {
        let mut params = Params::new()
            .set("username", &req.account)
            .set("password", &req.password)
            .opt("isuid", req.by.isuid())
            .opt("ip", req.ip.as_deref().filter(|s| !s.is_empty()))
            .flag("nolog", req.nolog);
        if let Some(q) = &req.question {
            params = params
                .set("checkques", 1)
                .set("questionid", q.question_id)
                .set("answer", &q.answer);
        }
        let data = self
            .t
            .call("/user/login", &params)
            .await?
            .into_data("/user/login")?;
        let status = data
            .get("status")
            .and_then(as_i64)
            .ok_or_else(|| UcenterError::Protocol("登录响应缺少 status".to_string()))?;
        let text = |key: &str| data.get(key).and_then(as_string).unwrap_or_default();
        Ok(match status {
            uid if uid > 0 => LoginOutcome::Success {
                uid,
                username: text("username"),
                email: text("email"),
            },
            -1 => LoginOutcome::UserNotFound,
            -2 => LoginOutcome::WrongPassword,
            -3 => LoginOutcome::QuestionMismatch,
            -4 => LoginOutcome::TooManyAttempts,
            other => LoginOutcome::Other(other),
        })
    }

    /// 查询该用户名在此 IP 下剩余的登录尝试次数，0 表示暂时禁止登录。
    pub async fn login_check(&self, username: &str, ip: &str) -> Result<i64> {
        let params = Params::new().set("username", username).set("ip", ip);
        self.scalar("/user/logincheck", params).await
    }

    pub async fn get_user(&self, lookup: UserLookup<'_>) -> Result<Option<UcUser>> {
        let params = match lookup {
            UserLookup::Username(name) => Params::new().set("username", name),
            UserLookup::Uid(uid) => Params::new().set("username", uid).set("isuid", 1),
        };
        let mut obj = match self.t.call("/user/get_user", &params).await? {
            Response::Data(Value::Object(obj)) => obj,
            Response::Scalar(0) | Response::Data(_) => return Ok(None),
            other => {
                return Err(UcenterError::Protocol(format!(
                    "/user/get_user 返回了意外结果: {other:?}"
                )));
            }
        };
        let uid = obj
            .remove("uid")
            .as_ref()
            .and_then(as_i64)
            .ok_or_else(|| UcenterError::Protocol("用户数据缺少 uid".to_string()))?;
        let mut take = |key: &str| {
            obj.remove(key)
                .as_ref()
                .and_then(as_string)
                .unwrap_or_default()
        };
        let username = take("username");
        let email = take("email");
        Ok(Some(UcUser {
            uid,
            username,
            email,
            extra: obj,
        }))
    }

    /// 注册成功返回新 UID。
    pub async fn register(
        &self,
        req: &RegisterRequest,
    ) -> Result<std::result::Result<i64, UserCheckError>> {
        let mut params = Params::new()
            .set("username", &req.username)
            .set("password", &req.password)
            .set("email", &req.email)
            .opt("regip", req.ip.as_deref().filter(|s| !s.is_empty()));
        params = with_question(params, req.question.as_ref());
        params = with_mobile(params, req.mobile.as_ref());
        let code = self.scalar("/user/register", params).await?;
        Ok(positive_or_check(code))
    }

    pub async fn check_username(
        &self,
        username: &str,
    ) -> Result<std::result::Result<(), UserCheckError>> {
        let code = self
            .scalar(
                "/user/check_username",
                Params::new().set("username", username),
            )
            .await?;
        Ok(positive_or_check(code).map(|_| ()))
    }

    pub async fn check_email(
        &self,
        email: &str,
    ) -> Result<std::result::Result<(), UserCheckError>> {
        let code = self
            .scalar("/user/check_email", Params::new().set("email", email))
            .await?;
        Ok(positive_or_check(code).map(|_| ()))
    }

    /// 手机号已被使用时返回 `true`。
    pub async fn check_mobile_exists(&self, mobile: &Mobile) -> Result<bool> {
        let code = self
            .scalar(
                "/user/check_secmobile",
                with_mobile(Params::new(), Some(mobile)),
            )
            .await?;
        Ok(code > 0)
    }

    pub async fn edit(&self, req: &EditRequest) -> Result<EditOutcome> {
        let mut params = Params::new()
            .set("username", &req.username)
            .opt("oldpw", req.old_password.as_deref())
            .opt("newpw", req.new_password.as_deref())
            .opt("email", req.email.as_deref())
            .flag("ignoreoldpw", req.ignore_old_password);
        params = with_question(params, req.question.as_ref());
        params = with_mobile(params, req.mobile.as_ref());
        let code = self.scalar("/user/edit", params).await?;
        Ok(EditOutcome::from_code(code))
    }

    /// 修改扩展字段，返回受影响行数；字段名须在 UCenter 扩展字段中配置。
    pub async fn edit_fields(&self, username: &str, fields: &[(&str, &str)]) -> Result<i64> {
        let mut params = Params::new().set("username", username);
        for (key, value) in fields {
            params = params.set(key, value);
        }
        self.scalar("/user/edit_field", params).await
    }

    pub async fn change_username(
        &self,
        uid: i64,
        new_username: &str,
    ) -> Result<std::result::Result<(), UserCheckError>> {
        let params = Params::new()
            .set("uid", uid)
            .set("newusername", new_username);
        let code = self.scalar("/user/chgusername", params).await?;
        Ok(positive_or_check(code).map(|_| ()))
    }

    /// 删除用户，返回删除数量。
    pub async fn delete(&self, uids: &[i64]) -> Result<i64> {
        let params = Params::new().list("uid", uids.iter().copied());
        self.scalar("/user/delete", params).await
    }

    pub async fn get_protected(&self) -> Result<Vec<ProtectedUser>> {
        let data = self
            .t
            .call("/user/getprotected", &Params::new())
            .await?
            .into_value();
        let rows = data.as_array().cloned().unwrap_or_default();
        Ok(rows
            .iter()
            .filter_map(|row| {
                Some(ProtectedUser {
                    uid: row.get("uid").and_then(as_i64)?,
                    username: row.get("username").and_then(as_string)?,
                })
            })
            .collect())
    }

    pub async fn add_protected(&self, usernames: &[&str], admin: &str) -> Result<bool> {
        let params = Params::new()
            .list("username", usernames.iter())
            .set("admin", admin);
        Ok(self.scalar("/user/addprotected", params).await? > 0)
    }

    pub async fn delete_protected(&self, usernames: &[&str]) -> Result<bool> {
        let params = Params::new().list("username", usernames.iter());
        Ok(self.scalar("/user/deleteprotected", params).await? > 0)
    }

    /// 将应用内的重名用户合并为 UCenter 用户，成功返回 UID。
    pub async fn merge(
        &self,
        req: &MergeRequest,
    ) -> Result<std::result::Result<i64, UserCheckError>> {
        let params = Params::new()
            .set("oldusername", &req.old_username)
            .set("newusername", &req.new_username)
            .set("uid", req.uid)
            .set("password", &req.password)
            .set("email", &req.email);
        let code = self.scalar("/user/merge", params).await?;
        Ok(positive_or_check(code))
    }

    pub async fn merge_remove(&self, username: &str) -> Result<()> {
        self.t
            .call(
                "/user/merge_remove",
                &Params::new().set("username", username),
            )
            .await?;
        Ok(())
    }

    /// 通过 UCenter 读取另一应用中该用户的积分，返回目标应用的原始输出。
    pub async fn get_credit(&self, app_id: i64, uid: i64, credit: i64) -> Result<String> {
        let params = Params::new()
            .set("appid", app_id)
            .set("uid", uid)
            .set("credit", credit);
        self.t
            .call("/user/getcredit", &params)
            .await?
            .into_text("/user/getcredit")
    }

    /// 同步登录脚本（HTML），应用未开启同步登录时为空串。
    pub async fn synlogin(&self, uid: i64) -> Result<String> {
        self.t
            .call("/user/synlogin", &Params::new().set("uid", uid))
            .await?
            .into_text("/user/synlogin")
    }

    /// 同步退出脚本（HTML）。
    pub async fn synlogout(&self) -> Result<String> {
        self.t
            .call("/user/synlogout", &Params::new())
            .await?
            .into_text("/user/synlogout")
    }

    async fn scalar(&self, action: &str, params: Params) -> Result<i64> {
        self.t.call(action, &params).await?.into_scalar(action)
    }
}

fn with_question(params: Params, question: Option<&SecurityQuestion>) -> Params {
    match question {
        Some(q) => params
            .set("questionid", q.question_id)
            .set("answer", &q.answer),
        None => params,
    }
}

fn with_mobile(params: Params, mobile: Option<&Mobile>) -> Params {
    match mobile {
        Some(m) => params
            .set("secmobicc", &m.country_code)
            .set("secmobile", &m.number),
        None => params,
    }
}
