//! 头像：上传（`/user/rectavatar`）、删除（`/user/deleteavatar`）、
//! 读取地址与下载（公开的 `avatar.php`，不需要签名）。
//!
//! 上传说明（`upload/control/user.php` 的 `onrectavatar`）：
//! - 一次提交大、中、小三张已裁剪好的图片，UCenter 不做缩放；
//! - 尺寸上限：大图 200×250，中图 120×120，小图 48×48，超限会整体拒绝；
//! - 支持 GIF、JPEG、PNG；
//! - RESTful 方式只能写入默认的“虚拟头像”（`avatartype` 只能从查询串传入，
//!   而查询串已被用作接口路径）。

use reqwest::header::CONTENT_TYPE;

use crate::error::{Result, UcenterError};
use crate::params::Params;
use crate::transport::Transport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarSize {
    Big,
    Middle,
    Small,
}

impl AvatarSize {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Big => "big",
            Self::Middle => "middle",
            Self::Small => "small",
        }
    }

    /// UCenter 接受的最大宽高。
    pub fn max_dimensions(self) -> (u32, u32) {
        match self {
            Self::Big => (200, 250),
            Self::Middle => (120, 120),
            Self::Small => (48, 48),
        }
    }
}

/// 头像类型：`Virtual` 为默认头像，`Real` 为真实头像。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarKind {
    #[default]
    Virtual,
    Real,
}

/// 上传用的三张图片（原始文件字节）。
#[derive(Debug, Clone)]
pub struct AvatarImages {
    pub big: Vec<u8>,
    pub middle: Vec<u8>,
    pub small: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarImage {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

pub struct AvatarApi<'a> {
    pub(crate) t: &'a Transport,
}

impl AvatarApi<'_> {
    /// 上传头像，成功返回 `true`；尺寸或格式不符时返回 `false`。
    pub async fn upload(&self, uid: i64, images: &AvatarImages) -> Result<bool> {
        for (name, bytes) in [
            ("big", &images.big),
            ("middle", &images.middle),
            ("small", &images.small),
        ] {
            if bytes.is_empty() {
                return Err(UcenterError::Config(format!("头像 {name} 图片为空")));
            }
        }
        let params = Params::new()
            .set("uid", uid)
            .set("avatar1", flash_hex(&images.big))
            .set("avatar2", flash_hex(&images.middle))
            .set("avatar3", flash_hex(&images.small));
        let body = self
            .t
            .call("/user/rectavatar", &params)
            .await?
            .into_text("/user/rectavatar")?;
        parse_rect_result(&body)
    }

    /// 删除头像。
    pub async fn delete(&self, uids: &[i64]) -> Result<()> {
        let params = Params::new().list("uid", uids.iter().copied());
        self.t.call("/user/deleteavatar", &params).await?;
        Ok(())
    }

    /// 公开头像地址。`ts` 为真时地址附带文件修改时间，便于浏览器缓存失效。
    pub fn url(&self, uid: i64, size: AvatarSize, kind: AvatarKind, ts: bool) -> String {
        let mut url = format!(
            "{}/avatar.php?uid={uid}&size={}",
            self.t.site_root(),
            size.as_str()
        );
        if kind == AvatarKind::Real {
            url.push_str("&type=real");
        }
        if ts {
            url.push_str("&ts=1");
        }
        url
    }

    /// 该用户是否已设置头像。
    pub async fn exists(&self, uid: i64, size: AvatarSize, kind: AvatarKind) -> Result<bool> {
        let url = format!("{}&check_file_exists=1", self.url(uid, size, kind, false));
        let (_, body) = self.t.get_public(&url).await?;
        Ok(body.trim_ascii() == b"1")
    }

    /// 下载头像；用户未设置头像时返回 `None`（不返回默认占位图）。
    pub async fn download(
        &self,
        uid: i64,
        size: AvatarSize,
        kind: AvatarKind,
    ) -> Result<Option<AvatarImage>> {
        if !self.exists(uid, size, kind).await? {
            return Ok(None);
        }
        let (headers, bytes) = self.t.get_public(&self.url(uid, size, kind, true)).await?;
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        Ok(Some(AvatarImage {
            bytes,
            content_type,
        }))
    }
}

/// `flashdata_decode` 使用的编码：每字节两位大写十六进制。
fn flash_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02X}");
    }
    out
}

fn parse_rect_result(body: &str) -> Result<bool> {
    if body.contains(r#"success="1""#) {
        return Ok(true);
    }
    if body.contains(r#"success="0""#) {
        return Ok(false);
    }
    if let Some(start) = body.find(r#"value=""#) {
        let rest = &body[start + 7..];
        let code = rest.split('"').next().unwrap_or("");
        return Err(UcenterError::Protocol(format!("上传头像失败（{code}）")));
    }
    Err(UcenterError::Protocol(format!(
        "无法识别的头像上传结果: {body}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_hex_matches_php_decoder() {
        assert_eq!(flash_hex(&[0x00, 0x0f, 0xab, 0xff]), "000FABFF");
    }

    #[test]
    fn parses_rect_results() {
        assert!(
            parse_rect_result(r#"<?xml version="1.0" ?><root><face success="1"/></root>"#).unwrap()
        );
        assert!(!parse_rect_result(r#"<root><face success="0"/></root>"#).unwrap());
        assert!(parse_rect_result(r#"<root><message type="error" value="-2" /></root>"#).is_err());
    }
}
