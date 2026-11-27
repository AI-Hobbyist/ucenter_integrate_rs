mod common;

use common::{MockUc, field};
use serde_json::json;
use ucenter_connector::{ApiErrorCode, Params, Response, UcenterClient, UcenterConfig};

#[tokio::test]
async fn token_is_issued_once_and_reused() {
    let uc = MockUc::start().await;
    uc.route("/user/check_username", |_| r#"{"ret":"1"}"#.to_string());
    let client = uc.client();

    for _ in 0..3 {
        let r = client
            .call("/user/check_username", Params::new())
            .await
            .unwrap();
        assert_eq!(r, Response::Scalar(1));
    }
    assert_eq!(uc.token_calls(), 1);
}

#[tokio::test]
async fn revoked_token_is_refreshed_and_retried_once() {
    let uc = MockUc::start().await;
    uc.route("/user/get_user", |form| {
        assert_eq!(field(form, "username"), Some("alice"));
        r#"{"ret":0,"data":{"uid":"9","username":"alice","email":"a@example.com"}}"#.to_string()
    });
    let client = uc.client();
    client.token().await.unwrap();
    uc.revoke_tokens();

    let r = client
        .call("user/get_user", Params::new().set("username", "alice"))
        .await
        .unwrap();
    assert_eq!(r.into_data("get_user").unwrap()["uid"], json!("9"));
    assert_eq!(uc.token_calls(), 2);
}

#[tokio::test]
async fn unpermitted_action_returns_api_error() {
    let uc = MockUc::start().await;
    let err = uc.client().call("/pm/ls", Params::new()).await.unwrap_err();
    assert_eq!(err.api_code(), Some(ApiErrorCode::ApiNotPermitted));
    assert!(err.api_code().unwrap().is_permission_error());
}

#[tokio::test]
async fn raw_text_output_is_preserved() {
    let uc = MockUc::start().await;
    uc.route("/user/synlogout", |_| String::new());
    let r = uc
        .client()
        .call("/user/synlogout", Params::new())
        .await
        .unwrap();
    assert_eq!(r, Response::Text(String::new()));
}

#[tokio::test]
async fn wrong_secret_reports_sign_error() {
    let uc = MockUc::start().await;
    let client =
        UcenterClient::new(UcenterConfig::new(&uc.base_url, common::APP_ID, "wrong")).unwrap();
    let err = client.check_connection().await.unwrap_err();
    assert_eq!(err.api_code(), Some(ApiErrorCode::SignInvalid));
    assert!(err.api_code().unwrap().is_credential_error());
}

#[tokio::test]
async fn invalid_config_is_rejected() {
    assert!(UcenterClient::new(UcenterConfig::new("uc.example.com", "a", "b")).is_err());
    assert!(UcenterClient::new(UcenterConfig::new("https://uc.example.com", "", "b")).is_err());
}
