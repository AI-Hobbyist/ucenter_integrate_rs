//! 本地模拟 UCenter RESTful 入口：校验签名与 Token，按动作返回预设正文。

#![allow(dead_code)]

use axum::Router;
use axum::extract::{Form, Query, RawQuery, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use ucenter_connector::{UcenterClient, UcenterConfig, sign};

pub const APP_ID: &str = "test-app";
pub const SECRET: &str = "test-secret";

pub type FormData = Vec<(String, String)>;
type Handler = Box<dyn Fn(&FormData) -> String + Send + Sync>;

#[derive(Default)]
pub struct MockState {
    token_calls: AtomicUsize,
    valid_tokens: Mutex<HashSet<String>>,
    routes: Mutex<HashMap<String, Handler>>,
    requests: Mutex<Vec<(String, FormData)>>,
}

#[derive(Clone)]
pub struct MockUc {
    pub base_url: String,
    state: Arc<MockState>,
}

impl MockUc {
    pub async fn start() -> Self {
        let state = Arc::new(MockState::default());
        let app = Router::new()
            .route("/api/index.php", post(handle))
            .route("/avatar.php", get(avatar))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            base_url: format!("http://{addr}/"),
            state,
        }
    }

    pub fn client(&self) -> UcenterClient {
        UcenterClient::new(UcenterConfig::new(&self.base_url, APP_ID, SECRET)).unwrap()
    }

    /// 为动作（如 `/user/login`）注册返回正文。
    pub fn route(
        &self,
        action: &str,
        handler: impl Fn(&FormData) -> String + Send + Sync + 'static,
    ) {
        self.state
            .routes
            .lock()
            .unwrap()
            .insert(action.to_string(), Box::new(handler));
    }

    /// 模拟 Redis 中的 Token 被清空。
    pub fn revoke_tokens(&self) {
        self.state.valid_tokens.lock().unwrap().clear();
    }

    pub fn token_calls(&self) -> usize {
        self.state.token_calls.load(Ordering::SeqCst)
    }

    /// 最近一次发往 `action` 的表单。
    pub fn last_form(&self, action: &str) -> FormData {
        self.state
            .requests
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(a, _)| a == action)
            .map(|(_, f)| f.clone())
            .unwrap_or_default()
    }
}

pub fn field<'a>(form: &'a FormData, key: &str) -> Option<&'a str> {
    form.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

pub fn fields<'a>(form: &'a FormData, key: &str) -> Vec<&'a str> {
    form.iter()
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .collect()
}

fn header<'a>(headers: &'a HeaderMap, key: &str) -> &'a str {
    headers.get(key).and_then(|v| v.to_str().ok()).unwrap_or("")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn handle(
    State(state): State<Arc<MockState>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    Form(form): Form<FormData>,
) -> String {
    let nonce = header(&headers, "nonce");
    let t = header(&headers, "t");
    if header(&headers, "appid") != APP_ID || nonce.is_empty() || t.is_empty() {
        return r#"{"ret":-101,"msg":"checkSign: param is missing"}"#.to_string();
    }
    if header(&headers, "sign") != sign(nonce, t, SECRET) {
        return r#"{"ret":-104,"msg":"checkSign: sign is invalid"}"#.to_string();
    }

    let action = query.unwrap_or_default();
    if action == "/token" {
        let n = state.token_calls.fetch_add(1, Ordering::SeqCst) + 1;
        let token = format!("TOKEN{n}");
        state.valid_tokens.lock().unwrap().insert(token.clone());
        return format!(
            r#"{{"ret":0,"token":"{token}","expires_in":{}}}"#,
            now() + 1800
        );
    }

    let token = header(&headers, "token");
    if token.is_empty() {
        return r#"{"ret":-105,"msg":"checkToken: token is missing"}"#.to_string();
    }
    if !state.valid_tokens.lock().unwrap().contains(token) {
        return r#"{"ret":-108,"msg":"getTokenData: token is error"}"#.to_string();
    }

    state
        .requests
        .lock()
        .unwrap()
        .push((action.clone(), form.clone()));
    match state.routes.lock().unwrap().get(&action) {
        Some(handler) => handler(&form),
        None => r#"{"ret":-114,"msg":"initParam: api is invalid"}"#.to_string(),
    }
}

/// 模拟头像 UID。
pub const AVATAR_UID: &str = "7";
pub const AVATAR_BYTES: &[u8] = b"\x89PNG-avatar";

/// 模拟 `avatar.php`：仅 UID 7 有头像。
async fn avatar(Query(q): Query<HashMap<String, String>>) -> axum::response::Response {
    let has_avatar = q.get("uid").map(String::as_str) == Some(AVATAR_UID);
    if q.contains_key("check_file_exists") {
        return (if has_avatar { "1" } else { "0" }).into_response();
    }
    if !has_avatar {
        return ([("content-type", "image/svg+xml")], "<svg/>").into_response();
    }
    ([("content-type", "image/png")], AVATAR_BYTES).into_response()
}
