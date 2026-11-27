# ucenter_connector

Reusable async Rust client for the Discuz! UCenter RESTful API.

## Usage

```toml
[dependencies]
ucenter_connector = { git = "https://github.com/AI-Hobbyist/ucenter_integrate_rs" }
```

```rust
use ucenter_connector::{UcenterClient, UcenterConfig};

let client = UcenterClient::new(UcenterConfig::new(
    "https://uc.example.com/",
    "123456",
    "your-secret",
))?;

let token_info = client.check_connection().await?;
let login_res = client.login("username", "password", None).await?;
```

## License

MIT
