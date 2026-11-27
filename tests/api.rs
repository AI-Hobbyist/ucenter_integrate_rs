mod common;

use common::{AVATAR_BYTES, MockUc, field, fields};
use serde_json::json;
use ucenter_connector::api::feed::FeedItem;
use ucenter_connector::api::friend::FriendDirection;
use ucenter_connector::api::mail::MailRequest;
use ucenter_connector::api::pm::{PmRecipients, SendPm};
use ucenter_connector::{
    ApiErrorCode, AuthorizeOutcome, AvatarImages, AvatarKind, AvatarSize, EditOutcome, EditRequest,
    LoginBy, LoginOutcome, LoginRequest, Mobile, RegisterRequest, SecurityQuestion, UserCheckError,
    UserLookup,
};

#[tokio::test]
async fn login_maps_status_and_sends_options() {
    let uc = MockUc::start().await;
    uc.route("/user/login", |form| {
        let body = match field(form, "password") {
            Some("right") => {
                r#"{"ret":0,"data":{"status":"12","username":"miku","email":"m@example.com"}}"#
            }
            Some("locked") => r#"{"ret":0,"data":{"status":-4}}"#,
            _ => r#"{"ret":0,"data":{"status":-2,"username":"miku","email":""}}"#,
        };
        body.to_string()
    });
    let user = uc.client();
    let user = user.user();

    let req = LoginRequest::new("m@example.com", "right")
        .by(LoginBy::Email)
        .ip("203.0.113.5")
        .question(SecurityQuestion {
            question_id: 2,
            answer: "blue".to_string(),
        });
    assert_eq!(
        user.login(&req).await.unwrap(),
        LoginOutcome::Success {
            uid: 12,
            username: "miku".to_string(),
            email: "m@example.com".to_string(),
        }
    );
    let form = uc.last_form("/user/login");
    assert_eq!(field(&form, "isuid"), Some("2"));
    assert_eq!(field(&form, "ip"), Some("203.0.113.5"));
    assert_eq!(field(&form, "checkques"), Some("1"));
    assert_eq!(field(&form, "questionid"), Some("2"));
    assert_eq!(field(&form, "nolog"), None);

    let wrong = user.login(&LoginRequest::new("miku", "bad")).await.unwrap();
    assert_eq!(wrong, LoginOutcome::WrongPassword);
    assert_eq!(field(&uc.last_form("/user/login"), "isuid"), None);

    let locked = user
        .login(&LoginRequest::new("miku", "locked"))
        .await
        .unwrap();
    assert_eq!(locked, LoginOutcome::TooManyAttempts);
}

#[tokio::test]
async fn get_user_keeps_extra_fields_and_handles_missing() {
    let uc = MockUc::start().await;
    uc.route("/user/get_user", |form| {
        if field(form, "isuid") == Some("1") && field(form, "username") == Some("5") {
            r#"{"ret":0,"data":{"uid":"5","username":"rin","email":"r@example.com","nickname":"Rin"}}"#
                .to_string()
        } else {
            r#"{"ret":"0"}"#.to_string()
        }
    });
    let client = uc.client();
    let found = client
        .user()
        .get_user(UserLookup::Uid(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.uid, 5);
    assert_eq!(found.username, "rin");
    assert_eq!(found.extra.get("nickname"), Some(&json!("Rin")));

    let missing = client
        .user()
        .get_user(UserLookup::Username("nobody"))
        .await
        .unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn register_separates_business_codes_from_gateway_errors() {
    let uc = MockUc::start().await;
    uc.route("/user/register", |form| match field(form, "username") {
        Some("new") => r#"{"ret":"31"}"#.to_string(),
        Some("taken") => r#"{"ret":"-3"}"#.to_string(),
        _ => r#"{"ret":-116,"msg":"apiFreqCheck: out of frequency"}"#.to_string(),
    });
    let client = uc.client();
    let user = client.user();

    let mut req = RegisterRequest::new("new", "pw", "n@example.com").ip("198.51.100.1");
    req.mobile = Some(Mobile {
        country_code: "86".to_string(),
        number: "13800000000".to_string(),
    });
    assert_eq!(user.register(&req).await.unwrap(), Ok(31));
    let form = uc.last_form("/user/register");
    assert_eq!(field(&form, "regip"), Some("198.51.100.1"));
    assert_eq!(field(&form, "secmobicc"), Some("86"));

    let taken = user
        .register(&RegisterRequest::new("taken", "pw", "t@example.com"))
        .await
        .unwrap();
    assert_eq!(taken, Err(UserCheckError::UsernameExists));

    let err = user
        .register(&RegisterRequest::new("flood", "pw", "f@example.com"))
        .await
        .unwrap_err();
    assert_eq!(err.api_code(), Some(ApiErrorCode::RateLimited));
}

#[tokio::test]
async fn edit_maps_outcomes() {
    let uc = MockUc::start().await;
    uc.route("/user/edit", |form| match field(form, "oldpw") {
        Some("right") => r#"{"ret":"1"}"#.to_string(),
        Some("wrong") => r#"{"ret":"-1"}"#.to_string(),
        _ => r#"{"ret":"-6"}"#.to_string(),
    });
    let client = uc.client();
    let edit = |old: &str| EditRequest {
        username: "miku".to_string(),
        old_password: Some(old.to_string()),
        new_password: Some("new".to_string()),
        email: Some("m@example.com".to_string()),
        ..EditRequest::default()
    };
    assert_eq!(
        client.user().edit(&edit("right")).await.unwrap(),
        EditOutcome::Updated(1)
    );
    assert_eq!(
        client.user().edit(&edit("wrong")).await.unwrap(),
        EditOutcome::WrongOldPassword
    );
    assert_eq!(
        client.user().edit(&edit("other")).await.unwrap(),
        EditOutcome::Rejected(UserCheckError::EmailExists)
    );
}

#[tokio::test]
async fn synlogin_returns_raw_script() {
    let uc = MockUc::start().await;
    uc.route("/user/synlogin", |_| {
        r#"<script src="https://a.example.com/api/uc.php"></script>"#.to_string()
    });
    let html = uc.client().user().synlogin(3).await.unwrap();
    assert!(html.starts_with("<script"));
}

#[tokio::test]
async fn authorize_url_and_code_exchange() {
    let uc = MockUc::start().await;
    uc.route("/user/authorize", |form| {
        assert_eq!(field(form, "callback"), Some("https://site.example.com/cb"));
        r#"{"ret":0,"data":{"url":"https://uc.example.com/index.php?m=user&a=authorize"}}"#
            .to_string()
    });
    uc.route("/user/check_code", |form| match field(form, "code") {
        Some("good") => r#"{"ret":0,"data":{"uid":"8","username":"luka"}}"#.to_string(),
        _ => r#"{"ret":"-1"}"#.to_string(),
    });
    let client = uc.client();
    let url = client
        .authorize()
        .url("https://site.example.com/cb")
        .await
        .unwrap();
    assert!(url.contains("a=authorize"));
    assert_eq!(
        client.authorize().check_code("good").await.unwrap(),
        AuthorizeOutcome::Success {
            uid: 8,
            username: "luka".to_string()
        }
    );
    assert_eq!(
        client.authorize().check_code("bad").await.unwrap(),
        AuthorizeOutcome::InvalidCode
    );
}

#[tokio::test]
async fn avatar_upload_encodes_hex_and_reads_result() {
    let uc = MockUc::start().await;
    uc.route("/user/rectavatar", |form| {
        if field(form, "avatar3") == Some("0AFF") {
            r#"<?xml version="1.0" ?><root><face success="1"/></root>"#.to_string()
        } else {
            r#"<?xml version="1.0" ?><root><face success="0"/></root>"#.to_string()
        }
    });
    let client = uc.client();
    let images = AvatarImages {
        big: vec![1, 2],
        middle: vec![3],
        small: vec![0x0a, 0xff],
    };
    assert!(client.avatar().upload(7, &images).await.unwrap());
    let form = uc.last_form("/user/rectavatar");
    assert_eq!(field(&form, "uid"), Some("7"));
    assert_eq!(field(&form, "avatar1"), Some("0102"));

    let rejected = AvatarImages {
        small: vec![1],
        ..images.clone()
    };
    assert!(!client.avatar().upload(7, &rejected).await.unwrap());

    let empty = AvatarImages {
        big: Vec::new(),
        ..images
    };
    assert!(client.avatar().upload(7, &empty).await.is_err());
}

#[tokio::test]
async fn avatar_url_exists_and_download() {
    let uc = MockUc::start().await;
    uc.route("/user/deleteavatar", |_| String::new());
    let client = uc.client();
    let avatar = client.avatar();

    let url = avatar.url(7, AvatarSize::Big, AvatarKind::Real, true);
    assert_eq!(
        url,
        format!("{}avatar.php?uid=7&size=big&type=real&ts=1", uc.base_url)
    );

    assert!(
        avatar
            .exists(7, AvatarSize::Middle, AvatarKind::Virtual)
            .await
            .unwrap()
    );
    assert!(
        !avatar
            .exists(9, AvatarSize::Middle, AvatarKind::Virtual)
            .await
            .unwrap()
    );

    let image = avatar
        .download(7, AvatarSize::Middle, AvatarKind::Virtual)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(image.bytes, AVATAR_BYTES);
    assert_eq!(image.content_type.as_deref(), Some("image/png"));
    assert!(
        avatar
            .download(9, AvatarSize::Middle, AvatarKind::Virtual)
            .await
            .unwrap()
            .is_none()
    );

    avatar.delete(&[7, 8]).await.unwrap();
    assert_eq!(
        fields(&uc.last_form("/user/deleteavatar"), "uid[]"),
        vec!["7", "8"]
    );
}

#[tokio::test]
async fn pm_friend_mail_feed_send_expected_params() {
    let uc = MockUc::start().await;
    uc.route("/pm/sendpm", |_| r#"{"ret":"101"}"#.to_string());
    uc.route("/pm/ls", |_| {
        r#"{"ret":0,"data":{"count":"1","data":[{"pmid":"101"}]}}"#.to_string()
    });
    uc.route("/friend/totalnum", |_| r#"{"ret":"4"}"#.to_string());
    uc.route("/mail/add", |_| r#"{"ret":"55"}"#.to_string());
    uc.route("/feed/add", |_| r#"{"ret":"9"}"#.to_string());
    let client = uc.client();

    let pmid = client
        .pm()
        .send(&SendPm {
            from_uid: 1,
            to: PmRecipients::Usernames(vec!["miku".to_string(), "rin".to_string()]),
            subject: "hi".to_string(),
            message: "hello".to_string(),
            reply_pmid: None,
            kind: None,
        })
        .await
        .unwrap();
    assert_eq!(pmid, 101);
    let form = uc.last_form("/pm/sendpm");
    assert_eq!(field(&form, "msgto"), Some("miku,rin"));
    assert_eq!(field(&form, "isusername"), Some("1"));

    let list = client
        .pm()
        .list(&ucenter_connector::api::pm::PmListQuery {
            uid: 1,
            page: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list["data"][0]["pmid"], json!("101"));

    assert_eq!(
        client
            .friend()
            .total(1, FriendDirection::Mutual)
            .await
            .unwrap(),
        4
    );
    assert_eq!(
        field(&uc.last_form("/friend/totalnum"), "direction"),
        Some("3")
    );

    let mail_id = client
        .mail()
        .add(&MailRequest {
            uids: vec![1, 2],
            subject: "s".to_string(),
            message: "m".to_string(),
            html: true,
            ..MailRequest::default()
        })
        .await
        .unwrap();
    assert_eq!(mail_id, 55);
    let form = uc.last_form("/mail/add");
    assert_eq!(field(&form, "uids"), Some("1,2"));
    assert_eq!(field(&form, "htmlon"), Some("1"));

    let feed_id = client
        .feed()
        .add(&FeedItem {
            app_id: 2,
            uid: 1,
            username: "miku".to_string(),
            title_template: "{actor} 发布了作品".to_string(),
            title_data: vec![("actor".to_string(), "miku".to_string())],
            images: vec![("https://img.example.com/a.png".to_string(), String::new())],
            ..FeedItem::default()
        })
        .await
        .unwrap();
    assert_eq!(feed_id, 9);
    let form = uc.last_form("/feed/add");
    assert_eq!(field(&form, "title_data[actor]"), Some("miku"));
    assert_eq!(
        field(&form, "image_1"),
        Some("https://img.example.com/a.png")
    );
}

#[tokio::test]
async fn version_info_is_parsed() {
    let uc = MockUc::start().await;
    uc.route("/version/check", |_| {
        r#"{"ret":0,"data":{"file":"1.7.0","db":"1.7.0"}}"#.to_string()
    });
    let version = uc.client().system().version().await.unwrap();
    assert_eq!(version.file, "1.7.0");
    assert_eq!(version.db, "1.7.0");
}
