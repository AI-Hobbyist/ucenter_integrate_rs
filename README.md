# ucenter_connector

Async Rust client for the Discuz! UCenter RESTful API (`/api/index.php`).

The UCenter RESTful gateway exposes every `on*` action of `upload/control/*.php`. Which
actions actually work depends on what the UCenter admin grants to your app under
RESTful API management; ungranted actions return `ApiErrorCode::ApiNotPermitted` (-114).

## Install

```toml
[dependencies]
ucenter_connector = { git = "https://github.com/AI-Hobbyist/ucenter_integrate_rs" }
```

## Features

| Module | Entry | Actions |
| --- | --- | --- |
| user | `client.user()` | login (username / UID / email / mobile, security question, IP), logincheck, register, check_username, check_email, check_secmobile, get_user (with extra fields), edit, edit_field, chgusername, delete, protected users, merge, getcredit, synlogin, synlogout |
| authorize | `client.authorize()` | authorize URL, check_code |
| avatar | `client.avatar()` | rectavatar upload, deleteavatar, public `avatar.php` URL / exists / download |
| pm | `client.pm()` | check_newpm, sendpm, delete, deletechat, deleteuser, readstatus, ignore, ls, view, viewnum, viewnode, chat members, kick / append chat, blacklist get / set / add / delete |
| friend | `client.friend()` | add, delete, totalnum, ls |
| credit | `client.credit()` | request |
| feed | `client.feed()` | add, delete, get |
| mail | `client.mail()` | add |
| tag | `client.tag()` | gettag |
| system | `client.system()` | app ls, domain ls, version check |
| any | `client.call(action, Params)` | anything else, e.g. `/app/add` |

Transport behaviour:

- Signs every request: `Sign = base64(hex(sha256(Nonce + T + secret)))`.
- Caches the app token, refreshes 60 s before expiry, and merges concurrent refreshes.
- On token missing / expired / invalid (-105 / -106 / -108) it drops the cached token and retries once.
- Separates gateway errors (`UcenterError::Api`) from business result codes, so a rate limit is never mistaken for "username exists".
- Handles all three output shapes: `{"ret":0,"data":...}`, wrapped scalars `{"ret":"n"}`, and raw text (e.g. synlogin scripts).

## Usage

```rust
use ucenter_connector::{
    LoginBy, LoginOutcome, LoginRequest, RegisterRequest, UcenterClient, UcenterConfig,
    UserCheckError,
};

let client = UcenterClient::new(UcenterConfig::new(
    "https://uc.example.com/",
    "your-app-id",
    "your-secret",
))?;

client.check_connection().await?;

// Pass the client IP: UCenter only enforces failed-login limits when `ip` is set.
let req = LoginRequest::new("user@example.com", "password")
    .by(LoginBy::Email)
    .ip("203.0.113.5");
match client.user().login(&req).await? {
    LoginOutcome::Success { uid, username, .. } => println!("{uid} {username}"),
    LoginOutcome::UserNotFound | LoginOutcome::WrongPassword => println!("bad credentials"),
    other => println!("{other:?}"),
}

match client
    .user()
    .register(&RegisterRequest::new("name", "password", "a@example.com"))
    .await?
{
    Ok(uid) => println!("new uid {uid}"),
    Err(UserCheckError::UsernameExists) => println!("taken"),
    Err(other) => println!("{}", other.description()),
}
```

## Avatars

Upload goes through `/user/rectavatar`: send three pre-cropped images (big ≤ 200×250,
middle ≤ 120×120, small ≤ 48×48; GIF / JPEG / PNG). UCenter does not resize, and rejects
the whole upload if any image exceeds its limit. Over RESTful only the default (virtual)
avatar can be written.

Reading uses the public `avatar.php`, which needs no signature or grant.

```rust
use ucenter_connector::{AvatarImages, AvatarKind, AvatarSize};

// Push a site avatar to UCenter.
let ok = client
    .avatar()
    .upload(uid, &AvatarImages { big, middle, small })
    .await?;

// Pull from UCenter (None when the user has no avatar).
if let Some(image) = client
    .avatar()
    .download(uid, AvatarSize::Middle, AvatarKind::Virtual)
    .await?
{
    std::fs::write("avatar.png", image.bytes)?;
}

// Or link it directly; `ts=1` busts caches after an update.
let url = client.avatar().url(uid, AvatarSize::Small, AvatarKind::Virtual, true);
```

A runnable version is in [`examples/avatar.rs`](examples/avatar.rs):

```sh
UC_URL=https://uc.example.com UC_APPID=... UC_SECRET=... cargo run --example avatar -- pull 1 ./out
UC_URL=https://uc.example.com UC_APPID=... UC_SECRET=... cargo run --example avatar -- push 1 big.png middle.png small.png
```

## Not wrapped

- `user/uploadavatar` and `user/camera`: temp upload and HTML page for the legacy Flash cropper; use `avatar().upload` instead.
- `pm_client/*`, `seccode`: browser pages.
- `app/add`, `app/ucinfo`: install-time actions that need the founder password; use `client.call`.

## Tests

`cargo test` runs unit tests plus integration tests against a local mock UCenter
(signature check, token issue / revoke / retry, every module's request and response mapping).

## License

MIT
