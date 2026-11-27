//! UCenter 响应解析。
//!
//! RESTful 入口有三种输出（`upload/index.php`、`upload/api/index.php`）：
//! - 控制器返回数组：`{"ret":0,"data":...}`；
//! - 控制器返回数字：被包装为 `{"ret":"n"}`；
//! - 控制器返回非数字字符串：原样输出（例如同步登录脚本、空串）。
//!
//! 网关错误同样放在 `ret` 中（`{"ret":-1xx,"msg":...}`），按“带 `msg` 或 `ret <= -100`”识别。

use serde_json::{Map, Value};

use crate::error::{ApiErrorCode, Result, UcenterError};

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// `{"ret":0,"data":...}` 或其它带业务字段的对象。
    Data(Value),
    /// 控制器返回的数字结果码。
    Scalar(i64),
    /// 控制器直接输出的文本（可能为空）。
    Text(String),
}

impl Response {
    /// 解析响应正文；网关错误返回 `UcenterError::Api`。
    pub fn parse(body: &str) -> Result<Self> {
        let Ok(value) = serde_json::from_str::<Value>(body.trim()) else {
            return Ok(Self::Text(body.to_string()));
        };
        match value {
            Value::Object(obj) => Self::from_object(obj),
            Value::Number(_) | Value::String(_) => match as_i64(&value) {
                Some(n) => Ok(Self::Scalar(n)),
                None => Ok(Self::Text(body.to_string())),
            },
            other => Ok(Self::Data(other)),
        }
    }

    fn from_object(mut obj: Map<String, Value>) -> Result<Self> {
        let Some(ret_value) = obj.get("ret") else {
            return Ok(Self::Data(Value::Object(obj)));
        };
        let ret = as_i64(ret_value)
            .ok_or_else(|| UcenterError::Protocol(format!("无法解析 ret: {ret_value}")))?;
        let msg = obj.get("msg").and_then(Value::as_str).map(str::to_string);
        if ret != 0 && (msg.is_some() || ret <= -100) {
            return Err(UcenterError::Api {
                code: ApiErrorCode::from_code(ret),
                msg: msg.unwrap_or_default(),
            });
        }
        if ret == 0
            && let Some(data) = obj.remove("data")
        {
            return Ok(Self::Data(data));
        }
        if obj.len() > 1 {
            return Ok(Self::Data(Value::Object(obj)));
        }
        Ok(Self::Scalar(ret))
    }

    /// 取数组结果；数字或文本结果视为协议错误。
    pub fn into_data(self, action: &str) -> Result<Value> {
        match self {
            Self::Data(v) => Ok(v),
            other => Err(unexpected(action, "数据对象", &other)),
        }
    }

    /// 取数字结果码。
    pub fn into_scalar(self, action: &str) -> Result<i64> {
        match self {
            Self::Scalar(n) => Ok(n),
            Self::Text(ref t) if t.trim().parse::<i64>().is_ok() => Ok(t.trim().parse().unwrap()),
            other => Err(unexpected(action, "数字结果", &other)),
        }
    }

    /// 取文本结果；数字结果转为字符串。
    pub fn into_text(self, action: &str) -> Result<String> {
        match self {
            Self::Text(t) => Ok(t),
            Self::Scalar(n) => Ok(n.to_string()),
            other => Err(unexpected(action, "文本", &other)),
        }
    }

    /// 宽松取值：任意结果都转为 JSON。
    pub fn into_value(self) -> Value {
        match self {
            Self::Data(v) => v,
            Self::Scalar(n) => Value::from(n),
            Self::Text(t) => Value::String(t),
        }
    }
}

fn unexpected(action: &str, expected: &str, got: &Response) -> UcenterError {
    let kind = match got {
        Response::Data(_) => "数据对象",
        Response::Scalar(_) => "数字结果",
        Response::Text(_) => "文本",
    };
    UcenterError::Protocol(format!("{action} 应返回{expected}，实际为{kind}"))
}

/// 兼容数字与数字字符串。
pub fn as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
}

pub(crate) fn as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_array_result() {
        let r = Response::parse(r#"{"ret":0,"data":{"uid":"3"}}"#).unwrap();
        assert_eq!(r, Response::Data(json!({"uid":"3"})));
    }

    #[test]
    fn parses_wrapped_scalar_string() {
        assert_eq!(
            Response::parse(r#"{"ret":"42"}"#).unwrap(),
            Response::Scalar(42)
        );
        assert_eq!(
            Response::parse(r#"{"ret":"-3"}"#).unwrap(),
            Response::Scalar(-3)
        );
        assert_eq!(
            Response::parse(r#"{"ret":"0"}"#).unwrap(),
            Response::Scalar(0)
        );
    }

    #[test]
    fn gateway_error_with_msg_is_api_error() {
        let err = Response::parse(r#"{"ret":-114,"msg":"initParam: api is invalid"}"#).unwrap_err();
        assert_eq!(err.api_code(), Some(ApiErrorCode::ApiNotPermitted));
        let err = Response::parse(r#"{"ret":-200}"#).unwrap_err();
        assert_eq!(err.api_code(), Some(ApiErrorCode::RedisMissing));
    }

    #[test]
    fn raw_text_and_empty_body_are_text() {
        let html = r#"<script src="x"></script>"#;
        assert_eq!(
            Response::parse(html).unwrap(),
            Response::Text(html.to_string())
        );
        assert_eq!(Response::parse("").unwrap(), Response::Text(String::new()));
    }

    #[test]
    fn token_object_is_data() {
        let r = Response::parse(r#"{"ret":0,"token":"ABC","expires_in":1700000000}"#).unwrap();
        assert!(matches!(r, Response::Data(ref v) if v["token"] == "ABC"));
    }
}
