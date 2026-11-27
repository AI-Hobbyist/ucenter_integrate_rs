//! 头像同步与上传示例。
//!
//! 环境变量：`UC_URL`、`UC_APPID`、`UC_SECRET`。
//!
//! 从 UCenter 拉取头像到本地目录：
//!   cargo run --example avatar -- pull <uid> <输出目录>
//!
//! 把本地已裁剪好的三张图片上传到 UCenter（大图 ≤200×250，中图 ≤120×120，小图 ≤48×48）：
//!   cargo run --example avatar -- push <uid> <big.png> <middle.png> <small.png>

use std::path::Path;
use ucenter_connector::{
    ApiErrorCode, AvatarImages, AvatarKind, AvatarSize, UcenterClient, UcenterConfig, UcenterError,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = |key: &str| std::env::var(key).map_err(|_| format!("缺少环境变量 {key}"));
    let client = UcenterClient::new(UcenterConfig::new(
        env("UC_URL")?,
        env("UC_APPID")?,
        env("UC_SECRET")?,
    ))?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["pull", uid, dir] => pull(&client, uid.parse()?, Path::new(dir)).await?,
        ["push", uid, big, middle, small] => {
            let images = AvatarImages {
                big: std::fs::read(big)?,
                middle: std::fs::read(middle)?,
                small: std::fs::read(small)?,
            };
            push(&client, uid.parse()?, &images).await?;
        }
        _ => eprintln!("用法: avatar pull <uid> <目录> | avatar push <uid> <big> <middle> <small>"),
    }
    Ok(())
}

/// 同步：把 UCenter 的三种尺寸头像保存到本地；用户未设置头像时跳过。
async fn pull(
    client: &UcenterClient,
    uid: i64,
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    for size in [AvatarSize::Big, AvatarSize::Middle, AvatarSize::Small] {
        match client
            .avatar()
            .download(uid, size, AvatarKind::Virtual)
            .await?
        {
            Some(image) => {
                let ext = match image.content_type.as_deref() {
                    Some("image/png") => "png",
                    Some("image/gif") => "gif",
                    _ => "jpg",
                };
                let path = dir.join(format!("{uid}_{}.{ext}", size.as_str()));
                std::fs::write(&path, &image.bytes)?;
                println!("已保存 {}", path.display());
            }
            None => println!("UID {uid} 没有 {} 头像", size.as_str()),
        }
    }
    // 前端也可以直接引用公开地址，ts=1 会在头像更新后自动刷新缓存。
    println!(
        "公开地址: {}",
        client
            .avatar()
            .url(uid, AvatarSize::Middle, AvatarKind::Virtual, true)
    );
    Ok(())
}

/// 上传：需要 UCenter 后台为应用开通 `/user/rectavatar`。
async fn push(
    client: &UcenterClient,
    uid: i64,
    images: &AvatarImages,
) -> Result<(), Box<dyn std::error::Error>> {
    match client.avatar().upload(uid, images).await {
        Ok(true) => println!("上传成功"),
        Ok(false) => println!("UCenter 拒绝了图片：请检查格式与尺寸上限"),
        Err(UcenterError::Api {
            code: ApiErrorCode::ApiNotPermitted,
            ..
        }) => println!("UCenter 后台未给该应用开通 /user/rectavatar"),
        Err(err) => return Err(err.into()),
    }
    Ok(())
}
