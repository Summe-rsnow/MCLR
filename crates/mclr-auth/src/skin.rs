//! Minecraft skin head cropping, upload, and cape management.

use crate::error::AuthError;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::imageops::FilterType;
use image::{GenericImageView, ImageFormat, Rgba, RgbaImage};
use mclr_core::paths;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::LazyLock;

const STEVE_BYTES: &[u8] = include_bytes!("../assets/steve.png");
const ALEX_BYTES: &[u8] = include_bytes!("../assets/alex.png");

static STEVE_HEAD: LazyLock<String> = LazyLock::new(|| {
    let png = crop_head_png(STEVE_BYTES).expect("bundled steve skin is valid");
    format!("data:image/png;base64,{}", BASE64.encode(&png))
});

static ALEX_HEAD: LazyLock<String> = LazyLock::new(|| {
    let png = crop_head_png(ALEX_BYTES).expect("bundled alex skin is valid");
    format!("data:image/png;base64,{}", BASE64.encode(&png))
});
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapeView {
    pub id: String,
    pub alias: String,
    pub url: String,
    pub active: bool,
}

/// Crops an 8x8 head with hat overlay from a 64x64 or 64x32 skin PNG,
/// scales it to 64x64 (nearest-neighbor), and returns the raw PNG bytes.
pub fn crop_head_png(skin_png_bytes: &[u8]) -> Result<Vec<u8>, AuthError> {
    let img = image::load_from_memory(skin_png_bytes)
        .map_err(|e| AuthError::SkinInvalidImage(format!("failed to load skin image: {e}")))?;

    let (width, height) = img.dimensions();
    if width != 64 || (height != 64 && height != 32) {
        return Err(AuthError::SkinInvalidImage(format!(
            "invalid skin dimensions: {width}x{height}, expected 64x64 or 64x32"
        )));
    }

    // 8x8 base face: x: 8..16, y: 8..16
    // 8x8 hat overlay: x: 40..48, y: 8..16
    let mut head = RgbaImage::new(8, 8);

    for y in 0..8 {
        for x in 0..8 {
            let face_px = img.get_pixel(8 + x, 8 + y);
            let hat_px = img.get_pixel(40 + x, 8 + y);

            let (rf, gf, bf, af) = (
                face_px[0] as f32,
                face_px[1] as f32,
                face_px[2] as f32,
                face_px[3] as f32 / 255.0,
            );
            let (rh, gh, bh, ah) = (
                hat_px[0] as f32,
                hat_px[1] as f32,
                hat_px[2] as f32,
                hat_px[3] as f32 / 255.0,
            );

            // Composite hat over face
            let a_out = ah + af * (1.0 - ah);
            let px = if a_out > 0.0 {
                let r_out = ((rh * ah + rf * af * (1.0 - ah)) / a_out)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                let g_out = ((gh * ah + gf * af * (1.0 - ah)) / a_out)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                let b_out = ((bh * ah + bf * af * (1.0 - ah)) / a_out)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                let a_byte = (a_out * 255.0).round().clamp(0.0, 255.0) as u8;
                Rgba([r_out, g_out, b_out, a_byte])
            } else {
                Rgba([0, 0, 0, 0])
            };

            head.put_pixel(x, y, px);
        }
    }

    // Resize 8x8 to 64x64 using nearest neighbor
    let scaled = image::imageops::resize(&head, 64, 64, FilterType::Nearest);

    let mut buf = Cursor::new(Vec::new());
    scaled
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| AuthError::SkinInvalidImage(format!("failed to encode head PNG: {e}")))?;
    Ok(buf.into_inner())
}

/// Returns a base64 data URL for default Steve or Alex skin head.
pub fn default_head_data_url(model: &str) -> String {
    if model == "slim" {
        ALEX_HEAD.clone()
    } else {
        STEVE_HEAD.clone()
    }
}

/// Crops an 8x8 head with hat overlay from a 64x64 or 64x32 skin PNG,
/// scales it to 64x64 (nearest-neighbor), saves to `cache/skin/head/<id>.png`,
/// and returns the `data:image/png;base64,...` data URL.
pub fn crop_skin_head(skin_png_bytes: &[u8], account_id: &str) -> Result<String, AuthError> {
    let png_bytes = crop_head_png(skin_png_bytes)?;

    // Cache to cache/skin/head/<id>.png
    let head_dir = paths::cache_dir().join("skin").join("head");
    if let Err(e) = fs::create_dir_all(&head_dir) {
        tracing::warn!("Failed to create head cache dir {:?}: {e}", head_dir);
    } else {
        let dest = head_dir.join(format!("{account_id}.png"));
        if let Err(e) = fs::write(&dest, &png_bytes) {
            tracing::warn!("Failed to write head cache {:?}: {e}", dest);
        }
    }

    let b64 = BASE64.encode(&png_bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// Generates a high-resolution head avatar from a skin PNG following the
/// PCL-CE "皮肤头像生成器" algorithm:
///
/// - The 8x8 face region (`8,8`) is upscaled to `size - size / 8` and inset by
///   `size / 16` on all sides, leaving a brim for the hair/hat overlay.
/// - The 8x8 hat overlay region (`40,8`) is upscaled to the full canvas and
///   alpha-composited on top, so brims extend past the face.
///
/// Accepts 64x64 / 64x32 skins and any integer upscale of them (128x128, 256x128, …).
pub fn generate_head_avatar(skin_png_bytes: &[u8], size: u32) -> Result<Vec<u8>, AuthError> {
    let img = image::load_from_memory(skin_png_bytes)
        .map_err(|e| AuthError::SkinInvalidImage(format!("failed to load skin image: {e}")))?;

    let (width, height) = img.dimensions();
    let valid_shape = width >= 64 && width % 64 == 0 && (height == width || height * 2 == width);
    if !valid_shape {
        return Err(AuthError::SkinInvalidImage(format!(
            "invalid skin dimensions: {width}x{height}, expected 64x64 / 64x32 or integer upscales"
        )));
    }

    let out_size = size.clamp(16, 1024);
    let scale = width / 64;
    let face_span = out_size - out_size / 8;
    let inset = out_size / 16;
    let mut out = RgbaImage::new(out_size, out_size);

    // 1. Face layer: nearest-neighbor upscale of the 8x8 face block.
    for dy in 0..face_span {
        let sy = 8 * scale + (dy * 8 / face_span);
        for dx in 0..face_span {
            let sx = 8 * scale + (dx * 8 / face_span);
            out.put_pixel(inset + dx, inset + dy, img.get_pixel(sx, sy));
        }
    }

    // 2. Hat/hair overlay: full-canvas upscale, alpha-composited over the face.
    for dy in 0..out_size {
        let sy = 8 * scale + (dy * 8 / out_size);
        for dx in 0..out_size {
            let sx = 40 * scale + (dx * 8 / out_size);
            let hair = img.get_pixel(sx, sy);
            let ha = hair[3] as f32 / 255.0;
            if ha <= 0.0 {
                continue;
            }
            if ha >= 1.0 {
                out.put_pixel(dx, dy, hair);
                continue;
            }
            let below = *out.get_pixel(dx, dy);
            let ba = below[3] as f32 / 255.0;
            let a_out = ha + ba * (1.0 - ha);
            if a_out <= 0.0 {
                continue;
            }
            let blend = |hc: u8, bc: u8| -> u8 {
                (((hc as f32 * ha) + (bc as f32 * ba * (1.0 - ha))) / a_out)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            out.put_pixel(
                dx,
                dy,
                Rgba([
                    blend(hair[0], below[0]),
                    blend(hair[1], below[1]),
                    blend(hair[2], below[2]),
                    (a_out * 255.0).round().clamp(0.0, 255.0) as u8,
                ]),
            );
        }
    }

    let mut buf = Cursor::new(Vec::new());
    out.write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| AuthError::SkinInvalidImage(format!("failed to encode avatar PNG: {e}")))?;
    Ok(buf.into_inner())
}

/// Uploads a custom skin to Minecraft services.
pub async fn upload_skin(
    client: &reqwest::Client,
    access_token: &str,
    variant: &str,
    png_path: &Path,
) -> Result<(), AuthError> {
    let bytes = fs::read(png_path)?;
    if bytes.len() > 24 * 1024 {
        return Err(AuthError::SkinInvalidImage(format!(
            "skin file too large: {} bytes, max is 24 KB",
            bytes.len()
        )));
    }

    let img = image::load_from_memory(&bytes)
        .map_err(|e| AuthError::SkinInvalidImage(format!("invalid skin image: {e}")))?;
    let (w, h) = img.dimensions();
    if w != 64 || (h != 64 && h != 32) {
        return Err(AuthError::SkinInvalidImage(format!(
            "invalid skin dimensions: {w}x{h}, expected 64x64 or 64x32"
        )));
    }

    let part = Part::bytes(bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| AuthError::SkinInvalidImage(e.to_string()))?;

    let form = Form::new()
        .text("variant", variant.to_string())
        .part("file", part);

    let res = client
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| AuthError::Http(format!("skin upload request failed: {e}")))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AuthError::Http(format!(
            "skin upload failed with status {status}: {body}"
        )));
    }

    Ok(())
}

/// Activates or clears a cape on Minecraft services.
pub async fn set_active_cape(
    client: &reqwest::Client,
    access_token: &str,
    cape_id: Option<&str>,
) -> Result<(), AuthError> {
    let url = "https://api.minecraftservices.com/minecraft/profile/capes/active";
    let res = match cape_id {
        Some(id) => {
            let body = serde_json::json!({ "capeId": id });
            client
                .put(url)
                .bearer_auth(access_token)
                .json(&body)
                .send()
                .await
        }
        None => client.delete(url).bearer_auth(access_token).send().await,
    }
    .map_err(|e| AuthError::Http(format!("cape request failed: {e}")))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AuthError::Http(format!(
            "cape operation failed with status {status}: {body}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop_skin_head_in_memory() {
        // Create 64x64 RGBA image
        let mut skin = RgbaImage::new(64, 64);
        // Paint face region (8..16, 8..16) red
        for y in 8..16 {
            for x in 8..16 {
                skin.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        // Paint hat region (40..48, 8..16) blue with 50% opacity
        for y in 8..16 {
            for x in 40..48 {
                skin.put_pixel(x, y, Rgba([0, 0, 255, 128]));
            }
        }

        let mut buf = Cursor::new(Vec::new());
        skin.write_to(&mut buf, ImageFormat::Png).unwrap();
        let bytes = buf.into_inner();

        let data_url = crop_skin_head(&bytes, "test-account").unwrap();
        assert!(data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_crop_skin_head_invalid_dimensions() {
        let skin = RgbaImage::new(32, 32);
        let mut buf = Cursor::new(Vec::new());
        skin.write_to(&mut buf, ImageFormat::Png).unwrap();
        let bytes = buf.into_inner();

        let res = crop_skin_head(&bytes, "test-account");
        assert!(res.is_err());
    }
    #[test]
    fn test_generate_head_avatar_composites_hair_over_face() {
        let mut skin = RgbaImage::new(64, 64);
        for y in 8..16 {
            for x in 8..16 {
                skin.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        // Opaque blue hat covering only the top-left of the hat block.
        for y in 8..12 {
            for x in 40..44 {
                skin.put_pixel(x, y, Rgba([0, 0, 255, 255]));
            }
        }

        let mut buf = Cursor::new(Vec::new());
        skin.write_to(&mut buf, ImageFormat::Png).unwrap();
        let png = generate_head_avatar(&buf.into_inner(), 64).unwrap();

        let out = image::load_from_memory(&png).unwrap();
        assert_eq!(out.dimensions(), (64, 64));
        // Top-left corner is hat-covered.
        let hat_px = out.get_pixel(1, 1);
        assert_eq!((hat_px[0], hat_px[1], hat_px[2]), (0, 0, 255));
        // Center of the face stays red.
        let face_px = out.get_pixel(32, 32);
        assert_eq!((face_px[0], face_px[1], face_px[2]), (255, 0, 0));
    }

    #[test]
    fn test_generate_head_avatar_respects_requested_size_and_rejects_bad_shape() {
        let skin = RgbaImage::new(128, 64);
        let mut buf = Cursor::new(Vec::new());
        skin.write_to(&mut buf, ImageFormat::Png).unwrap();
        let bytes = buf.into_inner();

        let png = generate_head_avatar(&bytes, 256).unwrap();
        assert_eq!(
            image::load_from_memory(&png).unwrap().dimensions(),
            (256, 256)
        );

        let bad = RgbaImage::new(100, 100);
        let mut bad_buf = Cursor::new(Vec::new());
        bad.write_to(&mut bad_buf, ImageFormat::Png).unwrap();
        assert!(generate_head_avatar(&bad_buf.into_inner(), 64).is_err());
    }

    #[test]
    fn test_default_head_data_url() {
        let classic = default_head_data_url("classic");
        let slim = default_head_data_url("slim");
        assert!(classic.starts_with("data:image/png;base64,"));
        assert!(slim.starts_with("data:image/png;base64,"));
        assert_ne!(classic, slim);
    }
}
