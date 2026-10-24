//! CurseForge v1 REST API client.

use std::time::Duration;

use mclr_core::secrets::{KEY_CURSEFORGE_API_KEY, get_secret};
use mclr_core::settings::SourcePriority;
use mclr_net::mirror::{SourceKind, racing_ladder, rewrite_url};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::CompError;
use crate::model::{
    CompDependency, CompDependencyKind, CompFile, CompFileHashes, CompPlatform, CompProject,
    CompReleaseChannel, CompSearchQuery, CompSearchResult, CompSort, CompType,
};

#[derive(Debug, Clone)]
pub struct CurseForgeClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    priority: SourcePriority,
}

impl Default for CurseForgeClient {
    fn default() -> Self {
        Self::new(SourcePriority::default())
    }
}

impl CurseForgeClient {
    pub fn new(priority: SourcePriority) -> Self {
        let mut default_headers = HeaderMap::new();
        let user_agent = format!("MCLR/{}", env!("CARGO_PKG_VERSION"));
        if let Ok(val) = HeaderValue::from_str(&user_agent) {
            default_headers.insert(USER_AGENT, val);
        }

        let client = Client::builder()
            .default_headers(default_headers)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        let api_key = get_secret(KEY_CURSEFORGE_API_KEY);

        Self {
            client,
            base_url: "https://api.curseforge.com".to_string(),
            api_key,
            priority,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_api_key(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key;
        self
    }

    pub fn with_priority(mut self, priority: SourcePriority) -> Self {
        self.priority = priority;
        self
    }

    fn require_api_key(&self) -> Result<&str, CompError> {
        match self.api_key.as_deref() {
            Some(key) if !key.trim().is_empty() => Ok(key.trim()),
            _ => Err(CompError::NotConfigured),
        }
    }

    fn resolve_candidate_urls(&self, endpoint_url: &Url) -> Vec<Url> {
        if self.base_url.trim_end_matches('/') == "https://api.curseforge.com" {
            let rewritten = rewrite_url(endpoint_url, SourceKind::CurseForgeApi);
            if rewritten.is_empty() {
                return vec![endpoint_url.clone()];
            }
            let ladder = racing_ladder(self.priority);
            let mut ordered = Vec::new();
            for (ladder_origin, _) in ladder {
                for (origin, u) in &rewritten {
                    if *origin == ladder_origin && !ordered.contains(u) {
                        ordered.push(u.clone());
                    }
                }
            }
            for (_, u) in &rewritten {
                if !ordered.contains(u) {
                    ordered.push(u.clone());
                }
            }
            if !ordered.is_empty() {
                return ordered;
            }
        }
        vec![endpoint_url.clone()]
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        path_and_query: &str,
    ) -> Result<T, CompError> {
        let key = self.require_api_key()?;
        let full = format!("{}{}", self.base_url.trim_end_matches('/'), path_and_query);
        let url = Url::parse(&full).map_err(|e| CompError::InvalidParams(e.to_string()))?;
        let candidates = self.resolve_candidate_urls(&url);

        let mut last_err = None;
        for cand in candidates {
            match self.client.get(cand).header("x-api-key", key).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let bytes = resp.bytes().await?;
                        return serde_json::from_slice(&bytes).map_err(CompError::Json);
                    }
                    last_err = Some(CompError::InvalidParams(format!("HTTP {}", resp.status())));
                }
                Err(e) => {
                    last_err = Some(CompError::Http(e));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| CompError::InvalidParams("Empty candidate URLs".into())))
    }

    async fn post_json<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, CompError> {
        let key = self.require_api_key()?;
        let full = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let url = Url::parse(&full).map_err(|e| CompError::InvalidParams(e.to_string()))?;
        let candidates = self.resolve_candidate_urls(&url);

        let mut last_err = None;
        for cand in candidates {
            match self
                .client
                .post(cand)
                .header("x-api-key", key)
                .json(body)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let bytes = resp.bytes().await?;
                        return serde_json::from_slice(&bytes).map_err(CompError::Json);
                    }
                    last_err = Some(CompError::InvalidParams(format!("HTTP {}", resp.status())));
                }
                Err(e) => {
                    last_err = Some(CompError::Http(e));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| CompError::InvalidParams("Empty candidate URLs".into())))
    }

    pub async fn search(&self, q: &CompSearchQuery) -> Result<CompSearchResult, CompError> {
        let _ = self.require_api_key()?;
        let class_id = match q.comp_type {
            CompType::Mod => 6,
            CompType::Modpack => 4471,
            CompType::Resourcepack => 12,
            CompType::Shader => 6552,
            CompType::Datapack => 6945,
            CompType::World => 17,
        };
        let mod_loader_type = match q.loader.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("forge") => 1,
            Some("liteloader") => 3,
            Some("fabric") => 4,
            Some("quilt") => 5,
            Some("neoforge") => 6,
            _ => 0,
        };
        let sort_field = match q.sort {
            CompSort::Relevance | CompSort::Follows => 2,
            CompSort::Updated => 3,
            CompSort::Downloads => 6,
            CompSort::Newest => 11,
        };
        let page_size = if q.limit == 0 {
            40
        } else {
            q.limit.clamp(1, 50)
        };

        let mut url = Url::parse("https://api.curseforge.com/v1/mods/search")
            .map_err(|e| CompError::InvalidParams(e.to_string()))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("gameId", "432");
            pairs.append_pair("classId", &class_id.to_string());
            if mod_loader_type > 0 {
                pairs.append_pair("modLoaderType", &mod_loader_type.to_string());
            }
            if let Some(mc) = &q.mc_version
                && !mc.is_empty()
            {
                pairs.append_pair("gameVersion", mc);
            }
            if !q.query.trim().is_empty() {
                pairs.append_pair("searchFilter", q.query.trim());
            }
            pairs.append_pair("sortField", &sort_field.to_string());
            pairs.append_pair("sortOrder", "desc");
            pairs.append_pair("pageSize", &page_size.to_string());
            pairs.append_pair("index", &q.offset.to_string());
        }

        let path_and_query = match url.query() {
            Some(qs) => format!("{}?{}", url.path(), qs),
            None => url.path().to_string(),
        };

        let resp: CfSearchResponse = self.get_json(&path_and_query).await?;
        let hits = resp.data.into_iter().map(map_cf_mod).collect();
        let total_hits = resp.pagination.map(|p| p.total_count).unwrap_or(0);

        Ok(CompSearchResult {
            hits,
            total_hits,
            offset: q.offset,
            limit: page_size,
        })
    }

    pub async fn get_project(&self, mod_id: &str) -> Result<CompProject, CompError> {
        let resp: CfSingleModResponse = self.get_json(&format!("/v1/mods/{mod_id}")).await?;
        let mut proj = map_cf_mod(resp.data);

        if let Ok(desc_resp) = self
            .get_json::<CfStringResponse>(&format!("/v1/mods/{mod_id}/description"))
            .await
        {
            proj.description = Some(desc_resp.data);
        }

        Ok(proj)
    }

    pub async fn get_files(
        &self,
        mod_id: &str,
        mc_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<CompFile>, CompError> {
        let mut url = Url::parse(&format!(
            "https://api.curseforge.com/v1/mods/{mod_id}/files"
        ))
        .map_err(|e| CompError::InvalidParams(e.to_string()))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("pageSize", "10000");
            if let Some(mc) = mc_version
                && !mc.is_empty()
            {
                pairs.append_pair("gameVersion", mc);
            }
            let loader_id = match loader.map(|s| s.to_lowercase()).as_deref() {
                Some("forge") => 1,
                Some("liteloader") => 3,
                Some("fabric") => 4,
                Some("quilt") => 5,
                Some("neoforge") => 6,
                _ => 0,
            };
            if loader_id > 0 {
                pairs.append_pair("modLoaderType", &loader_id.to_string());
            }
        }

        let path_and_query = match url.query() {
            Some(qs) => format!("{}?{}", url.path(), qs),
            None => url.path().to_string(),
        };

        let resp: CfFilesResponse = self.get_json(&path_and_query).await?;
        Ok(resp.data.into_iter().map(map_cf_file).collect())
    }

    pub async fn get_mods_batch(&self, mod_ids: &[u64]) -> Result<Vec<CompProject>, CompError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            mod_ids: &'a [u64],
        }
        let resp: CfModsBatchResponse = self.post_json("/v1/mods", &Body { mod_ids }).await?;
        Ok(resp.data.into_iter().map(map_cf_mod).collect())
    }

    pub async fn get_files_batch(&self, file_ids: &[u64]) -> Result<Vec<CompFile>, CompError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            file_ids: &'a [u64],
        }
        let resp: CfFilesResponse = self.post_json("/v1/mods/files", &Body { file_ids }).await?;
        Ok(resp.data.into_iter().map(map_cf_file).collect())
    }

    pub async fn match_fingerprints(
        &self,
        fingerprints: &[u32],
    ) -> Result<Vec<CompFile>, CompError> {
        #[derive(Serialize)]
        struct Body<'a> {
            fingerprints: &'a [u32],
        }
        let resp: CfFingerprintsResponse = self
            .post_json("/v1/fingerprints/432", &Body { fingerprints })
            .await?;
        Ok(resp
            .data
            .exact_matches
            .into_iter()
            .map(|m| map_cf_file(m.file))
            .collect())
    }
}

// ============================================================================
// Wire structs & Mappers
// ============================================================================

#[derive(Debug, Deserialize)]
struct CfPagination {
    #[serde(rename = "totalCount", default)]
    total_count: u32,
}

#[derive(Debug, Deserialize)]
struct CfSearchResponse {
    #[serde(default)]
    data: Vec<CfMod>,
    pagination: Option<CfPagination>,
}

#[derive(Debug, Deserialize)]
struct CfSingleModResponse {
    data: CfMod,
}

#[derive(Debug, Deserialize)]
struct CfModsBatchResponse {
    #[serde(default)]
    data: Vec<CfMod>,
}

#[derive(Debug, Deserialize)]
struct CfStringResponse {
    data: String,
}

#[derive(Debug, Deserialize)]
struct CfFilesResponse {
    #[serde(default)]
    data: Vec<CfFile>,
}

#[derive(Debug, Deserialize)]
struct CfFingerprintsResponse {
    data: CfFingerprintsData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFingerprintsData {
    #[serde(default)]
    exact_matches: Vec<CfFingerprintMatch>,
}

#[derive(Debug, Deserialize)]
struct CfFingerprintMatch {
    file: CfFile,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: u64,
    #[serde(default)]
    class_id: Option<u32>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    thumbs_up_count: u64,
    logo: Option<CfLogo>,
    #[serde(default)]
    authors: Vec<CfAuthor>,
    #[serde(default)]
    categories: Vec<CfCategory>,
    #[serde(default)]
    latest_files_indexes: Vec<CfFileIndex>,
    #[serde(default)]
    date_modified: String,
    links: Option<CfLinks>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLogo {
    thumbnail_url: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CfCategory {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFileIndex {
    #[serde(default)]
    game_version: String,
    mod_loader: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLinks {
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: u64,
    mod_id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    release_type: u32,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    file_length: u64,
    download_url: Option<String>,
    allow_mod_distribution: Option<bool>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    hashes: Vec<CfHash>,
    #[serde(default)]
    dependencies: Vec<CfDependency>,
    file_fingerprint: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CfHash {
    value: String,
    algo: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfDependency {
    mod_id: u64,
    relation_type: u32,
}

fn map_cf_mod(m: CfMod) -> CompProject {
    let comp_type = match m.class_id {
        Some(4471) => CompType::Modpack,
        Some(12) => CompType::Resourcepack,
        Some(6552) => CompType::Shader,
        Some(6945) => CompType::Datapack,
        Some(17) => CompType::World,
        _ => CompType::Mod,
    };

    let mut mc_versions = Vec::new();
    let mut loaders = Vec::new();
    for idx in m.latest_files_indexes {
        if !idx.game_version.is_empty() && !mc_versions.contains(&idx.game_version) {
            mc_versions.push(idx.game_version);
        }
        if let Some(loader_id) = idx.mod_loader {
            let l_name = match loader_id {
                1 => Some("forge"),
                3 => Some("liteloader"),
                4 => Some("fabric"),
                5 => Some("quilt"),
                6 => Some("neoforge"),
                _ => None,
            };
            if let Some(ln) = l_name {
                let s = ln.to_string();
                if !loaders.contains(&s) {
                    loaders.push(s);
                }
            }
        }
    }

    let author = m
        .authors
        .first()
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let icon_url = m.logo.and_then(|l| l.thumbnail_url.or(l.url));
    let categories = m.categories.into_iter().map(|c| c.name).collect();
    let website_url = m.links.and_then(|l| l.website_url);

    CompProject {
        platform: CompPlatform::Curseforge,
        id: m.id.to_string(),
        slug: m.slug,
        title: m.name,
        summary: m.summary.clone(),
        description: Some(m.summary),
        icon_url,
        author,
        downloads: m.download_count as u64,
        follows: m.thumbs_up_count,
        categories,
        mc_versions,
        loaders,
        comp_type,
        updated_at: m.date_modified,
        website_url,
    }
}

fn map_cf_file(f: CfFile) -> CompFile {
    let channel = match f.release_type {
        2 => CompReleaseChannel::Beta,
        3 => CompReleaseChannel::Alpha,
        _ => CompReleaseChannel::Release,
    };

    let mut mc_versions = Vec::new();
    let mut loaders = Vec::new();
    for gv in f.game_versions {
        let lower = gv.to_lowercase();
        match lower.as_str() {
            "forge" | "fabric" | "neoforge" | "quilt" | "liteloader" | "rift" => {
                if !loaders.contains(&lower) {
                    loaders.push(lower);
                }
            }
            "client" | "server" | "java 8" | "java 16" | "java 17" | "java 21" => {}
            _ => {
                if !mc_versions.contains(&gv) {
                    mc_versions.push(gv);
                }
            }
        }
    }

    let mut sha1 = None;
    for h in f.hashes {
        if h.algo == 1 {
            sha1 = Some(h.value);
        }
    }

    let dependencies = f
        .dependencies
        .into_iter()
        .filter_map(|d| {
            let kind = match d.relation_type {
                3 => Some(CompDependencyKind::Required),
                2 => Some(CompDependencyKind::Optional),
                5 => Some(CompDependencyKind::Incompatible),
                1 => Some(CompDependencyKind::Embedded),
                _ => None,
            }?;
            Some(CompDependency {
                project_id: d.mod_id.to_string(),
                kind,
            })
        })
        .collect();

    let (download_urls, distribution_disabled) = match &f.download_url {
        Some(u) if !u.trim().is_empty() => {
            let urls = rewrite_cdn_urls(u);
            (urls, false)
        }
        _ => {
            if f.allow_mod_distribution == Some(false) {
                (Vec::new(), true)
            } else {
                // Edge fallback: https://edge.forgecdn.net/files/{id/1000}/{id%1000}/{fileName}
                let major = f.id / 1000;
                let minor = f.id % 1000;
                let fallback = format!(
                    "https://edge.forgecdn.net/files/{major}/{minor}/{}",
                    f.file_name
                );
                (rewrite_cdn_urls(&fallback), false)
            }
        }
    };

    CompFile {
        id: f.id.to_string(),
        project_id: f.mod_id.to_string(),
        platform: CompPlatform::Curseforge,
        name: if f.display_name.is_empty() {
            f.file_name.clone()
        } else {
            f.display_name
        },
        file_name: f.file_name,
        mc_versions,
        loaders,
        channel,
        release_date: f.file_date,
        size: f.file_length,
        hashes: CompFileHashes {
            sha1,
            sha512: None,
            murmur2: f.file_fingerprint,
        },
        download_urls,
        distribution_disabled,
        dependencies,
    }
}

fn rewrite_cdn_urls(raw_url: &str) -> Vec<String> {
    if let Ok(parsed) = Url::parse(raw_url) {
        let rewritten = rewrite_url(&parsed, SourceKind::ModCdn);
        if !rewritten.is_empty() {
            return rewritten.into_iter().map(|(_, u)| u.to_string()).collect();
        }
    }
    vec![raw_url.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclr_core::AppError;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_missing_api_key_returns_not_configured() {
        let client = CurseForgeClient::new(SourcePriority::OfficialFirst).with_api_key(None);
        let q = CompSearchQuery {
            platform: CompPlatform::Curseforge,
            comp_type: CompType::Mod,
            query: "jei".to_string(),
            mc_version: Some("1.20.1".to_string()),
            loader: Some("forge".to_string()),
            category: None,
            sort: CompSort::Relevance,
            offset: 0,
            limit: 20,
        };
        let err = client.search(&q).await.unwrap_err();
        assert_eq!(err.code(), "notConfigured");
        let app_err: AppError = err.into();
        assert_eq!(app_err.code, "notConfigured");
        assert_eq!(
            app_err.params.get("what").map(String::as_str),
            Some("curseforge")
        );
    }

    #[tokio::test]
    async fn test_curseforge_search_and_files_wiremock() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/mods/search"))
            .and(header("x-api-key", "test-cf-key"))
            .and(query_param("gameId", "432"))
            .and(query_param("classId", "6"))
            .and(query_param("searchFilter", "jei"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "id": 238222,
                    "classId": 6,
                    "name": "Just Enough Items (JEI)",
                    "slug": "jei",
                    "summary": "View Items and Recipes",
                    "downloadCount": 300000000.0,
                    "thumbsUpCount": 12000,
                    "authors": [{ "name": "mezz" }],
                    "categories": [{ "name": "Map and Information" }],
                    "latestFilesIndexes": [{ "gameVersion": "1.20.1", "modLoader": 1 }],
                    "dateModified": "2024-05-01T00:00:00Z",
                    "links": { "websiteUrl": "https://www.curseforge.com/minecraft/mc-mods/jei" }
                }],
                "pagination": { "totalCount": 1 }
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1/mods/238222/files"))
            .and(header("x-api-key", "test-cf-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {
                        "id": 4593548,
                        "modId": 238222,
                        "displayName": "jei-1.20.1-forge-15.2.0.27.jar",
                        "fileName": "jei-1.20.1-forge-15.2.0.27.jar",
                        "releaseType": 1,
                        "fileDate": "2024-01-01T00:00:00Z",
                        "fileLength": 1150000,
                        "downloadUrl": null,
                        "allowModDistribution": true,
                        "gameVersions": ["1.20.1", "Forge"],
                        "hashes": [{ "value": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "algo": 1 }],
                        "dependencies": [{ "modId": 306612, "relationType": 3 }],
                        "fileFingerprint": 123456789
                    },
                    {
                        "id": 4593549,
                        "modId": 238222,
                        "displayName": "restricted.jar",
                        "fileName": "restricted.jar",
                        "releaseType": 2,
                        "fileDate": "2024-01-02T00:00:00Z",
                        "fileLength": 50000,
                        "downloadUrl": null,
                        "allowModDistribution": false,
                        "gameVersions": ["1.20.1", "Forge"],
                        "hashes": [],
                        "dependencies": []
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = CurseForgeClient::new(SourcePriority::OfficialFirst)
            .with_base_url(server.uri())
            .with_api_key(Some("test-cf-key".to_string()));

        let q = CompSearchQuery {
            platform: CompPlatform::Curseforge,
            comp_type: CompType::Mod,
            query: "jei".to_string(),
            mc_version: Some("1.20.1".to_string()),
            loader: Some("forge".to_string()),
            category: None,
            sort: CompSort::Relevance,
            offset: 0,
            limit: 40,
        };
        let search_res = client.search(&q).await.expect("search cf");
        assert_eq!(search_res.total_hits, 1);
        assert_eq!(search_res.hits[0].slug, "jei");

        let files = client
            .get_files("238222", Some("1.20.1"), Some("forge"))
            .await
            .expect("get files");
        assert_eq!(files.len(), 2);

        // File 0: null downloadUrl with allowModDistribution=true builds edge fallback 4593/548
        assert!(!files[0].distribution_disabled);
        assert!(
            files[0]
                .download_urls
                .iter()
                .any(|u| u.contains("/files/4593/548/jei-1.20.1-forge-15.2.0.27.jar"))
        );
        assert_eq!(files[0].dependencies.len(), 1);
        assert_eq!(files[0].dependencies[0].kind, CompDependencyKind::Required);

        // File 1: allowModDistribution=false marks distribution_disabled=true
        assert!(files[1].distribution_disabled);
        assert!(files[1].download_urls.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_curseforge_search() {
        let client = CurseForgeClient::new(SourcePriority::MirrorFirst);
        let q = CompSearchQuery {
            platform: CompPlatform::Curseforge,
            comp_type: CompType::Mod,
            query: "jei".to_string(),
            mc_version: Some("1.20.1".to_string()),
            loader: Some("forge".to_string()),
            category: None,
            sort: CompSort::Relevance,
            offset: 0,
            limit: 10,
        };
        match client.search(&q).await {
            Ok(res) => {
                assert!(!res.hits.is_empty());
            }
            Err(CompError::NotConfigured) => {
                // Expected when MCLR_CURSEFORGE_API_KEY is not set in env
            }
            Err(other) => panic!("unexpected CurseForge live error: {other}"),
        }
    }
}
