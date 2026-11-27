//! 错误类型：配置、网络、协议错误，以及 UCenter RESTful 网关返回的接口错误码。

use std::fmt;

/// UCenter RESTful 网关错误码（`upload/lib/restful.class.php` 的 `Error` 表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorCode {
    /// -101 签名参数缺失
    SignParamMissing,
    /// -102 签名时应用无效
    SignAppInvalid,
    /// -103 签名已过期
    SignExpired,
    /// -104 签名不匹配
    SignInvalid,
    /// -105 未携带 Token
    TokenMissing,
    /// -106 Token 已过期
    TokenExpired,
    /// -107 Token 与 Appid 不匹配
    TokenAppMismatch,
    /// -108 Token 无效（不存在于 Redis）
    TokenInvalid,
    /// -109 接口参数无效
    ApiInvalid,
    /// -110 应用配置缺失
    AppConfigMissing,
    /// -111 应用通信密钥缺失
    SecretMissing,
    /// -112 接口地址为空
    ApiUrlEmpty,
    /// -113 接口地址错误
    ApiUrlInvalid,
    /// -114 接口未授权给该应用
    ApiNotPermitted,
    /// -115 接口权限检查失败
    ApiPermCheckFailed,
    /// -116 超过接口频率限制
    RateLimited,
    /// -117 脚本为空
    ScriptEmpty,
    /// -118 脚本格式错误
    ScriptInvalid,
    /// -119 回调授权令牌无效
    CallbackAuthInvalid,
    /// -200 UCenter 未连接 Redis
    RedisMissing,
    /// -201 请求路径无效
    InvalidRequest,
    /// -202 回调地址不在白名单
    CallbackInvalid,
    /// -999 Redis 扩展缺失
    RedisExtensionMissing,
    /// 其它未登记的错误码
    Other(i64),
}

impl ApiErrorCode {
    pub fn from_code(code: i64) -> Self {
        match code {
            -101 => Self::SignParamMissing,
            -102 => Self::SignAppInvalid,
            -103 => Self::SignExpired,
            -104 => Self::SignInvalid,
            -105 => Self::TokenMissing,
            -106 => Self::TokenExpired,
            -107 => Self::TokenAppMismatch,
            -108 => Self::TokenInvalid,
            -109 => Self::ApiInvalid,
            -110 => Self::AppConfigMissing,
            -111 => Self::SecretMissing,
            -112 => Self::ApiUrlEmpty,
            -113 => Self::ApiUrlInvalid,
            -114 => Self::ApiNotPermitted,
            -115 => Self::ApiPermCheckFailed,
            -116 => Self::RateLimited,
            -117 => Self::ScriptEmpty,
            -118 => Self::ScriptInvalid,
            -119 => Self::CallbackAuthInvalid,
            -200 => Self::RedisMissing,
            -201 => Self::InvalidRequest,
            -202 => Self::CallbackInvalid,
            -999 => Self::RedisExtensionMissing,
            other => Self::Other(other),
        }
    }

    pub fn code(self) -> i64 {
        match self {
            Self::SignParamMissing => -101,
            Self::SignAppInvalid => -102,
            Self::SignExpired => -103,
            Self::SignInvalid => -104,
            Self::TokenMissing => -105,
            Self::TokenExpired => -106,
            Self::TokenAppMismatch => -107,
            Self::TokenInvalid => -108,
            Self::ApiInvalid => -109,
            Self::AppConfigMissing => -110,
            Self::SecretMissing => -111,
            Self::ApiUrlEmpty => -112,
            Self::ApiUrlInvalid => -113,
            Self::ApiNotPermitted => -114,
            Self::ApiPermCheckFailed => -115,
            Self::RateLimited => -116,
            Self::ScriptEmpty => -117,
            Self::ScriptInvalid => -118,
            Self::CallbackAuthInvalid => -119,
            Self::RedisMissing => -200,
            Self::InvalidRequest => -201,
            Self::CallbackInvalid => -202,
            Self::RedisExtensionMissing => -999,
            Self::Other(code) => code,
        }
    }

    /// Token 缺失、过期或无效：清除缓存并重新签发后可重试。
    pub fn is_token_error(self) -> bool {
        matches!(
            self,
            Self::TokenMissing | Self::TokenExpired | Self::TokenInvalid
        )
    }

    /// 接口未在 UCenter 后台授权给当前应用。
    pub fn is_permission_error(self) -> bool {
        matches!(self, Self::ApiNotPermitted | Self::ApiPermCheckFailed)
    }

    /// 签名或应用配置问题（AppID、通信密钥、服务器时间）。
    pub fn is_credential_error(self) -> bool {
        matches!(
            self,
            Self::SignParamMissing
                | Self::SignAppInvalid
                | Self::SignExpired
                | Self::SignInvalid
                | Self::TokenAppMismatch
                | Self::AppConfigMissing
                | Self::SecretMissing
        )
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::SignParamMissing => "签名参数缺失",
            Self::SignAppInvalid => "AppID 无效",
            Self::SignExpired => "签名已过期，请检查服务器时间",
            Self::SignInvalid => "签名不匹配，请检查通信密钥",
            Self::TokenMissing => "缺少应用 Token",
            Self::TokenExpired => "应用 Token 已过期",
            Self::TokenAppMismatch => "应用 Token 与 AppID 不匹配",
            Self::TokenInvalid => "应用 Token 无效",
            Self::ApiInvalid => "接口参数无效",
            Self::AppConfigMissing => "UCenter 中未找到该应用配置",
            Self::SecretMissing => "UCenter 中该应用未设置通信密钥",
            Self::ApiUrlEmpty => "接口地址为空",
            Self::ApiUrlInvalid => "接口地址错误",
            Self::ApiNotPermitted => "该接口未授权给当前应用",
            Self::ApiPermCheckFailed => "接口权限检查失败",
            Self::RateLimited => "超过接口调用频率限制",
            Self::ScriptEmpty => "脚本为空",
            Self::ScriptInvalid => "脚本格式错误",
            Self::CallbackAuthInvalid => "回调授权令牌无效",
            Self::RedisMissing => "UCenter 未连接 Redis",
            Self::InvalidRequest => "请求路径无效",
            Self::CallbackInvalid => "回调地址不在白名单中",
            Self::RedisExtensionMissing => "UCenter 缺少 Redis 扩展",
            Self::Other(_) => "未知接口错误",
        }
    }
}

impl fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}（{}）", self.description(), self.code())
    }
}

#[derive(Debug)]
pub enum UcenterError {
    /// 本地配置不完整或无效。
    Config(String),
    /// 网络、TLS 或 HTTP 状态错误。
    Http(String),
    /// 响应格式与预期不符。
    Protocol(String),
    /// UCenter RESTful 网关拒绝请求。
    Api { code: ApiErrorCode, msg: String },
}

impl UcenterError {
    pub fn api_code(&self) -> Option<ApiErrorCode> {
        match self {
            Self::Api { code, .. } => Some(*code),
            _ => None,
        }
    }
}

impl fmt::Display for UcenterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(m) => write!(f, "UCenter 配置错误: {m}"),
            Self::Http(m) => write!(f, "UCenter 网络错误: {m}"),
            Self::Protocol(m) => write!(f, "UCenter 协议错误: {m}"),
            Self::Api { code, msg } if msg.is_empty() => write!(f, "UCenter 错误: {code}"),
            Self::Api { code, msg } => write!(f, "UCenter 错误: {code}: {msg}"),
        }
    }
}

impl std::error::Error for UcenterError {}

pub type Result<T> = std::result::Result<T, UcenterError>;
