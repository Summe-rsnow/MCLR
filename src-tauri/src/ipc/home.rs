//! Minecraft news and announcements IPC commands.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use mclr_core::{AppError, SettingsStore, paths};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use url::Url;

/// Minecraft news item surfaced on the launcher homepage.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub date: String,
    pub category: String,
    pub image_url: Option<String>,
    pub article_url: String,
}

/// Optional action button for an announcement card.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementButton {
    pub text: String,
    pub url: Option<String>,
}

/// Announcement item displayed in the launcher announcement banner.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementItem {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub btn1: Option<AnnouncementButton>,
    pub btn2: Option<AnnouncementButton>,
}

const ALLOWED_NEWS_DOMAINS: &[&str] = &["minecraft.net", "minecraft-services.net", "microsoft.com"];

/// Validates whether a URL is a trusted Minecraft/Microsoft resource.
///
/// Requires scheme `https` and that the host is equal to or a subdomain of:
/// - `minecraft.net`
/// - `minecraft-services.net`
/// - `microsoft.com`
pub fn is_whitelisted_minecraft_url(url_str: &str) -> bool {
    let Ok(parsed) = Url::parse(url_str) else {
        return false;
    };

    if parsed.scheme() != "https" {
        return false;
    }

    let Some(host) = parsed.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();

    for &domain in ALLOWED_NEWS_DOMAINS {
        if host == domain || host.ends_with(&format!(".{domain}")) {
            return true;
        }
    }

    false
}

/// Upstream search API response schemas.
#[derive(Deserialize)]
struct UpstreamSearchResponse {
    result: Option<UpstreamSearchResult>,
}

#[derive(Deserialize)]
struct UpstreamSearchResult {
    #[serde(default)]
    results: Vec<UpstreamNewsEntry>,
}

#[derive(Deserialize)]
struct UpstreamNewsEntry {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    time: Option<serde_json::Value>,
    #[serde(rename = "publishDate", default)]
    publish_date: Option<String>,
    #[serde(rename = "type", default)]
    category: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    image: Option<serde_json::Value>,
}

fn sanitize_article_url(raw_url: &str) -> Option<String> {
    let resolved = if raw_url.starts_with('/') {
        format!("https://www.minecraft.net{raw_url}")
    } else {
        raw_url.to_string()
    };

    if is_whitelisted_minecraft_url(&resolved) {
        Some(resolved)
    } else {
        None
    }
}

fn sanitize_image_url(val: Option<&serde_json::Value>) -> Option<String> {
    let raw = match val {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Object(map)) => {
            if let Some(serde_json::Value::String(url)) = map.get("url") {
                url.clone()
            } else if let Some(serde_json::Value::String(src)) = map.get("src") {
                src.clone()
            } else {
                return None;
            }
        }
        _ => return None,
    };

    let resolved = if raw.starts_with('/') {
        format!("https://www.minecraft.net{raw}")
    } else {
        raw
    };

    if is_whitelisted_minecraft_url(&resolved) {
        Some(resolved)
    } else {
        None
    }
}

fn parse_news_entries(raw_json: &str) -> Vec<NewsItem> {
    let Ok(resp) = serde_json::from_str::<UpstreamSearchResponse>(raw_json) else {
        return Vec::new();
    };

    let Some(result) = resp.result else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for (idx, entry) in result.results.into_iter().enumerate() {
        let title = entry.title.unwrap_or_default().trim().to_string();
        if title.is_empty() {
            continue;
        }

        let summary = entry
            .description
            .or(entry.summary)
            .unwrap_or_default()
            .trim()
            .to_string();

        let article_url = entry
            .url
            .as_deref()
            .and_then(sanitize_article_url)
            .unwrap_or_else(|| "https://www.minecraft.net/zh-hans/articles".to_string());

        let image_url = sanitize_image_url(entry.image.as_ref());

        let date = if let Some(d) = entry.publish_date {
            d
        } else if let Some(time_val) = entry.time {
            match time_val {
                serde_json::Value::Number(n) => n
                    .as_i64()
                    .map(|secs| {
                        chrono::DateTime::from_timestamp(secs, 0)
                            .map(|dt| dt.format("%Y-%m-%d").to_string())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default(),
                serde_json::Value::String(s) => s,
                _ => String::new(),
            }
        } else {
            String::new()
        };

        let id = entry.id.unwrap_or_else(|| format!("news-{idx}"));
        let category = entry.category.unwrap_or_else(|| "News".to_string());

        items.push(NewsItem {
            id,
            title,
            summary,
            date,
            category,
            image_url,
            article_url,
        });
    }

    items
}

fn get_cache_path(page: u32) -> PathBuf {
    paths::cache_dir().join(format!("news_page_{page}.json"))
}

#[tauri::command]
#[specta::specta]
pub async fn news_fetch(
    http: State<'_, crate::http::HttpState>,
    page: u32,
    force_refresh: bool,
) -> Result<Vec<NewsItem>, AppError> {
    let cache_file = get_cache_path(page);

    let cache_for_read = cache_file.clone();
    if !force_refresh
        && let Ok(Some(items)) = super::run_blocking(move || {
            Ok::<_, AppError>((|| {
                let modified = fs::metadata(&cache_for_read).ok()?.modified().ok()?;
                if SystemTime::now().duration_since(modified).ok()? >= Duration::from_secs(30 * 60)
                {
                    return None;
                }
                let content = fs::read_to_string(&cache_for_read).ok()?;
                serde_json::from_str::<Vec<NewsItem>>(&content).ok()
            })())
        })
        .await
    {
        return Ok(items);
    }

    let client = http.client();

    let url = format!(
        "https://net-secondary.web.minecraft-services.net/api/v1.0/zh-cn/search?pageSize=24&sortType=Recent&category=News&newsOnly=true&page={page}"
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::new("home.newsFetchFailed").with_detail(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(
            AppError::new("home.newsFetchFailed").with_param("status", resp.status().as_str())
        );
    }

    let raw_text = resp
        .text()
        .await
        .map_err(|e| AppError::new("home.newsFetchFailed").with_detail(e.to_string()))?;

    let items = parse_news_entries(&raw_text);

    if let Ok(serialized) = serde_json::to_string(&items) {
        let _ = super::run_blocking(move || {
            let _ = fs::create_dir_all(paths::cache_dir());
            let _ = fs::write(&cache_file, serialized);
            Ok::<_, AppError>(())
        })
        .await;
    }

    Ok(items)
}

#[tauri::command]
#[specta::specta]
pub async fn announcements_fetch(
    http: State<'_, crate::http::HttpState>,
    settings_state: State<'_, SettingsStore>,
) -> Result<Vec<AnnouncementItem>, AppError> {
    let settings = settings_state.get().await;
    let announcement_url = settings.update.announcement_url.trim();

    if announcement_url.is_empty() {
        return Ok(Vec::new());
    }

    let client = http.client();
    let resp =
        client.get(announcement_url).send().await.map_err(|e| {
            AppError::new("home.announcementFetchFailed").with_detail(e.to_string())
        })?;

    if !resp.status().is_success() {
        return Err(AppError::new("home.announcementFetchFailed")
            .with_param("status", resp.status().as_str()));
    }

    let announcements: Vec<AnnouncementItem> = resp
        .json()
        .await
        .map_err(|e| AppError::new("home.announcementParseFailed").with_detail(e.to_string()))?;

    let dismissed = &settings.states.dismissed_announcements;
    let active = announcements
        .into_iter()
        .filter(|a| !dismissed.contains(&a.id))
        .collect();

    Ok(active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_whitelisted_minecraft_url() {
        assert!(is_whitelisted_minecraft_url(
            "https://minecraft.net/zh-hans/"
        ));
        assert!(is_whitelisted_minecraft_url(
            "https://www.minecraft.net/zh-hans/article/test"
        ));
        assert!(!is_whitelisted_minecraft_url(
            "https://launchercontent.mojang.com/news.jpg"
        ));
        assert!(is_whitelisted_minecraft_url(
            "https://net-secondary.web.minecraft-services.net/api/v1.0"
        ));
        assert!(is_whitelisted_minecraft_url(
            "https://sub.minecraft-services.net/path"
        ));
        assert!(is_whitelisted_minecraft_url("https://microsoft.com"));
        assert!(is_whitelisted_minecraft_url(
            "https://www.microsoft.com/store"
        ));
        assert!(!is_whitelisted_minecraft_url("http://minecraft.net"));
        assert!(!is_whitelisted_minecraft_url("https://evilminecraft.net"));
        assert!(!is_whitelisted_minecraft_url("https://not-microsoft.com"));
        assert!(!is_whitelisted_minecraft_url("not a url"));
    }

    #[test]
    fn test_parse_news_entries() {
        let sample_json = r#"{
            "result": {
                "results": [
                    {
                        "id": "item-1",
                        "title": "Minecraft 1.21.4 Release",
                        "description": "A new release is out now!",
                        "publishDate": "2026-02-15",
                        "type": "Game Update",
                        "url": "/zh-hans/article/minecraft-1-21-4",
                        "image": {
                            "url": "https://www.minecraft.net/content/dam/games/minecraft/key-art/1-21-4.jpg"
                        }
                    },
                    {
                        "title": "Untrusted Link News",
                        "description": "Notice something fishy",
                        "url": "https://external-domain.com/article",
                        "image": "https://external-image.com/pic.png"
                    }
                ]
            }
        }"#;

        let items = parse_news_entries(sample_json);
        assert_eq!(items.len(), 2);

        let item1 = &items[0];
        assert_eq!(item1.id, "item-1");
        assert_eq!(item1.title, "Minecraft 1.21.4 Release");
        assert_eq!(item1.summary, "A new release is out now!");
        assert_eq!(item1.date, "2026-02-15");
        assert_eq!(item1.category, "Game Update");
        assert_eq!(
            item1.article_url,
            "https://www.minecraft.net/zh-hans/article/minecraft-1-21-4"
        );
        assert_eq!(
            item1.image_url,
            Some(
                "https://www.minecraft.net/content/dam/games/minecraft/key-art/1-21-4.jpg"
                    .to_string()
            )
        );

        let item2 = &items[1];
        assert_eq!(item2.id, "news-1");
        // Untrusted article_url is sanitized back to default
        assert_eq!(
            item2.article_url,
            "https://www.minecraft.net/zh-hans/articles"
        );
        // Untrusted image_url is sanitized to None
        assert_eq!(item2.image_url, None);
    }
}
