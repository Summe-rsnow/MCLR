use std::collections::HashMap;
use std::time::Duration;

use mclr_core::settings::SourcePriority;
use mclr_net::mirror::{SourceKind, racing_ladder, rewrite_url};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;

use crate::error::CompError;
use crate::model::{
    CompDependency, CompDependencyKind, CompFile, CompFileHashes, CompPlatform, CompProject,
    CompReleaseChannel, CompSearchQuery, CompSearchResult, CompSort, CompType,
};

/// Modrinth REST API client.
#[derive(Debug, Clone)]
pub struct ModrinthClient {
    client: Client,
    base_url: String,
    priority: SourcePriority,
}

impl Default for ModrinthClient {
    fn default() -> Self {
        Self::new(SourcePriority::default())
    }
}

impl ModrinthClient {
    /// Creates a new `ModrinthClient` with the given `SourcePriority`.
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

        Self {
            client,
            base_url: "https://api.modrinth.com".to_string(),
            priority,
        }
    }

    /// Creates a client with a custom HTTP client and base URL (e.g. For wiremock tests).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Overrides the `SourcePriority`.
    pub fn with_priority(mut self, priority: SourcePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Returns candidate URLs according to `SourceKind::ModrinthApi` and `SourcePriority`
    /// when using the official API base, or returns the single URL when overridden (e.g. test mock server).
    fn resolve_candidate_urls(&self, endpoint_url: &Url) -> Vec<Url> {
        if self.base_url.trim_end_matches('/') == "https://api.modrinth.com" {
            let rewritten = rewrite_url(endpoint_url, SourceKind::ModrinthApi);
            if rewritten.is_empty() {
                return vec![endpoint_url.clone()];
            }

            let ladder = racing_ladder(self.priority);
            let mut ordered_urls = Vec::new();
            for (ladder_origin, _) in ladder {
                for (origin, candidate_url) in &rewritten {
                    if *origin == ladder_origin && !ordered_urls.contains(candidate_url) {
                        ordered_urls.push(candidate_url.clone());
                    }
                }
            }
            for (_, candidate_url) in &rewritten {
                if !ordered_urls.contains(candidate_url) {
                    ordered_urls.push(candidate_url.clone());
                }
            }
            if !ordered_urls.is_empty() {
                return ordered_urls;
            }
        }
        vec![endpoint_url.clone()]
    }

    /// Helper to execute GET requests across candidate URLs (failover / mirror priority).
    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint_url: &Url,
    ) -> Result<T, CompError> {
        let candidates = self.resolve_candidate_urls(endpoint_url);
        let mut last_err = None;

        for url in &candidates {
            debug!("Modrinth GET {}", url);
            let user_agent = format!("MCLR/{}", env!("CARGO_PKG_VERSION"));
            let req = self.client.get(url.clone()).header(USER_AGENT, &user_agent);
            match req.send().await {
                Ok(resp) => {
                    if resp.status() == reqwest::StatusCode::NOT_FOUND {
                        return Err(CompError::ProjectNotFound(url.to_string()));
                    }
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        last_err = Some(CompError::InvalidParams(format!("HTTP {status}: {text}")));
                        continue;
                    }
                    let body_bytes = resp.bytes().await?;
                    match serde_json::from_slice::<T>(&body_bytes) {
                        Ok(val) => return Ok(val),
                        Err(err) => {
                            last_err = Some(CompError::Json(err));
                        }
                    }
                }
                Err(err) => {
                    last_err = Some(CompError::Http(err));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| CompError::ProjectNotFound(endpoint_url.to_string())))
    }

    /// Helper to execute POST requests across candidate URLs.
    async fn post_json<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        endpoint_url: &Url,
        body: &B,
    ) -> Result<T, CompError> {
        let candidates = self.resolve_candidate_urls(endpoint_url);
        let mut last_err = None;

        for url in &candidates {
            debug!("Modrinth POST {}", url);
            let user_agent = format!("MCLR/{}", env!("CARGO_PKG_VERSION"));
            let req = self
                .client
                .post(url.clone())
                .header(USER_AGENT, &user_agent)
                .json(body);
            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        last_err = Some(CompError::InvalidParams(format!("HTTP {status}: {text}")));
                        continue;
                    }
                    let body_bytes = resp.bytes().await?;
                    match serde_json::from_slice::<T>(&body_bytes) {
                        Ok(val) => return Ok(val),
                        Err(err) => {
                            last_err = Some(CompError::Json(err));
                        }
                    }
                }
                Err(err) => {
                    last_err = Some(CompError::Http(err));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| CompError::InvalidParams("POST failed".to_string())))
    }

    /// Searches Modrinth projects.
    pub async fn search(&self, q: &CompSearchQuery) -> Result<CompSearchResult, CompError> {
        let mut url = Url::parse(&format!(
            "{}/v2/search",
            self.base_url.trim_end_matches('/')
        ))
        .map_err(|e| CompError::InvalidParams(e.to_string()))?;

        let limit = if q.limit == 0 { 40 } else { q.limit };
        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string());
        url.query_pairs_mut()
            .append_pair("offset", &q.offset.to_string());

        let index_str = match q.sort {
            CompSort::Relevance => "relevance",
            CompSort::Downloads => "downloads",
            CompSort::Follows => "follows",
            CompSort::Newest => "newest",
            CompSort::Updated => "updated",
        };
        url.query_pairs_mut().append_pair("index", index_str);

        if !q.query.trim().is_empty() {
            url.query_pairs_mut().append_pair("query", q.query.trim());
        }

        // Facets: JSON array of arrays
        // [ ["project_type:X"], ["categories:loader"], ["versions:mc"], ["categories:category"] ]
        let mut facets: Vec<Vec<String>> = Vec::new();

        let type_str = match q.comp_type {
            CompType::Mod => "mod",
            CompType::Modpack => "modpack",
            CompType::Resourcepack => "resourcepack",
            CompType::Shader => "shader",
            CompType::Datapack => "datapack",
            CompType::World => "datapack",
        };
        facets.push(vec![format!("project_type:{type_str}")]);

        if let Some(l) = &q.loader {
            let l_trimmed = l.trim().to_lowercase();
            if !l_trimmed.is_empty() {
                facets.push(vec![format!("categories:{l_trimmed}")]);
            }
        }

        if let Some(v) = &q.mc_version {
            let v_trimmed = v.trim();
            if !v_trimmed.is_empty() {
                facets.push(vec![format!("versions:{v_trimmed}")]);
            }
        }

        if let Some(c) = &q.category {
            let c_trimmed = c.trim().to_lowercase();
            if !c_trimmed.is_empty() {
                facets.push(vec![format!("categories:{c_trimmed}")]);
            }
        }

        let facets_json = serde_json::to_string(&facets).map_err(CompError::Json)?;
        url.query_pairs_mut().append_pair("facets", &facets_json);

        let resp: ModrinthSearchResponse = self.get_json(&url).await?;

        let hits = resp
            .hits
            .into_iter()
            .map(|h| h.into_comp_project())
            .collect();

        Ok(CompSearchResult {
            hits,
            total_hits: resp.total_hits,
            offset: resp.offset,
            limit: resp.limit,
        })
    }

    /// Gets project details by ID or slug.
    pub async fn get_project(&self, id_or_slug: &str) -> Result<CompProject, CompError> {
        let trimmed = id_or_slug.trim();
        if trimmed.is_empty() {
            return Err(CompError::InvalidParams(
                "project id/slug is empty".to_string(),
            ));
        }
        let mut url = Url::parse(&format!(
            "{}/v2/project",
            self.base_url.trim_end_matches('/')
        ))
        .map_err(|e| CompError::InvalidParams(e.to_string()))?;
        if let Ok(mut segments) = url.path_segments_mut() {
            segments.push(trimmed);
        }
        let raw: ModrinthRawProject = self.get_json(&url).await?;
        Ok(raw.into_comp_project())
    }

    /// Gets multiple projects by IDs/slugs.
    pub async fn get_projects_batch(&self, ids: &[String]) -> Result<Vec<CompProject>, CompError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids_json = serde_json::to_string(ids).map_err(CompError::Json)?;
        let mut url = Url::parse(&format!(
            "{}/v2/projects",
            self.base_url.trim_end_matches('/')
        ))
        .map_err(|e| CompError::InvalidParams(e.to_string()))?;
        url.query_pairs_mut().append_pair("ids", &ids_json);

        let raw_list: Vec<ModrinthRawProject> = self.get_json(&url).await?;
        Ok(raw_list
            .into_iter()
            .map(|p| p.into_comp_project())
            .collect())
    }

    /// Gets versions/files for a project, optionally filtered by Minecraft version and loader.
    pub async fn get_versions(
        &self,
        project_id: &str,
        mc_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<CompFile>, CompError> {
        let trimmed = project_id.trim();
        if trimmed.is_empty() {
            return Err(CompError::InvalidParams("project_id is empty".to_string()));
        }
        let mut url = Url::parse(&format!(
            "{}/v2/project",
            self.base_url.trim_end_matches('/')
        ))
        .map_err(|e| CompError::InvalidParams(e.to_string()))?;
        if let Ok(mut segments) = url.path_segments_mut() {
            segments.push(trimmed);
            segments.push("version");
        }
        url.query_pairs_mut()
            .append_pair("include_changelog", "false");
        if let Some(v) = mc_version {
            let v_trimmed = v.trim();
            if !v_trimmed.is_empty() {
                let json = serde_json::to_string(&vec![v_trimmed]).map_err(CompError::Json)?;
                url.query_pairs_mut().append_pair("game_versions", &json);
            }
        }

        if let Some(l) = loader {
            let l_trimmed = l.trim().to_lowercase();
            if !l_trimmed.is_empty() {
                let json = serde_json::to_string(&vec![l_trimmed]).map_err(CompError::Json)?;
                url.query_pairs_mut().append_pair("loaders", &json);
            }
        }

        let raw_versions: Vec<ModrinthRawVersion> = self.get_json(&url).await?;
        let files = raw_versions
            .into_iter()
            .filter_map(|v| v.into_comp_file(self.priority))
            .collect();
        Ok(files)
    }

    /// Look up versions/files by SHA1 hashes (`POST /v2/version_files`).
    pub async fn get_version_files_by_sha1(
        &self,
        hashes: &[String],
    ) -> Result<HashMap<String, CompFile>, CompError> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = Url::parse(&format!(
            "{}/v2/version_files",
            self.base_url.trim_end_matches('/')
        ))
        .map_err(|e| CompError::InvalidParams(e.to_string()))?;

        #[derive(Serialize)]
        struct VersionFilesRequest<'a> {
            hashes: &'a [String],
            algorithm: &'static str,
        }

        let req = VersionFilesRequest {
            hashes,
            algorithm: "sha1",
        };

        let map: HashMap<String, ModrinthRawVersion> = self.post_json(&url, &req).await?;
        let mut result = HashMap::new();
        for (hash, raw_ver) in map {
            if let Some(file) = raw_ver.into_comp_file(self.priority) {
                result.insert(hash, file);
            }
        }
        Ok(result)
    }

    /// Checks updates for versions/files (`POST /v2/version_files/update`).
    pub async fn get_version_files_update(
        &self,
        hashes: &[String],
        loaders: &[String],
        game_versions: &[String],
    ) -> Result<HashMap<String, CompFile>, CompError> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = Url::parse(&format!(
            "{}/v2/version_files/update",
            self.base_url.trim_end_matches('/')
        ))
        .map_err(|e| CompError::InvalidParams(e.to_string()))?;

        #[derive(Serialize)]
        struct UpdateRequest<'a> {
            hashes: &'a [String],
            algorithm: &'static str,
            loaders: &'a [String],
            game_versions: &'a [String],
        }

        let req = UpdateRequest {
            hashes,
            algorithm: "sha1",
            loaders,
            game_versions,
        };

        let map: HashMap<String, ModrinthRawVersion> = self.post_json(&url, &req).await?;
        let mut result = HashMap::new();
        for (hash, raw_ver) in map {
            if let Some(file) = raw_ver.into_comp_file(self.priority) {
                result.insert(hash, file);
            }
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Internal deserialization types for Modrinth JSON API
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthSearchHit>,
    offset: u32,
    limit: u32,
    total_hits: u32,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    #[serde(default)]
    categories: Vec<String>,
    project_type: String,
    downloads: u64,
    #[serde(default)]
    icon_url: Option<String>,
    author: String,
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    follows: u64,
    date_modified: String,
}

impl ModrinthSearchHit {
    fn into_comp_project(self) -> CompProject {
        let comp_type = parse_comp_type(&self.project_type, &self.categories);
        let website_url = format!("https://modrinth.com/{}/{}", self.project_type, self.slug);

        let mut loaders = Vec::new();
        let mut categories = Vec::new();
        for cat in self.categories {
            if is_loader(&cat) {
                if !loaders.contains(&cat) {
                    loaders.push(cat);
                }
            } else if !categories.contains(&cat) {
                categories.push(cat);
            }
        }
        let (zh_name, wiki_id) = if comp_type == CompType::Mod {
            match crate::modname::lookup_by_slug(&self.slug)
                .or_else(|| crate::modname::lookup_by_slug(&self.project_id))
            {
                Some(entry) => (
                    Some(crate::modname::display_zh(entry).to_string()),
                    Some(entry.wiki_id),
                ),
                None => (None, None),
            }
        } else {
            (None, None)
        };

        CompProject {
            platform: CompPlatform::Modrinth,
            id: self.project_id,
            slug: self.slug,
            title: self.title,
            summary: self.description,
            description: None,
            icon_url: self.icon_url,
            author: self.author,
            downloads: self.downloads,
            follows: self.follows,
            categories,
            mc_versions: self.versions,
            loaders,
            comp_type,
            updated_at: self.date_modified,
            website_url: Some(website_url),
            zh_name,
            wiki_id,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ModrinthRawProject {
    id: String,
    slug: String,
    title: String,
    description: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    additional_categories: Vec<String>,
    project_type: String,
    downloads: u64,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    team: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    followers: u64,
    updated: String,
}

impl ModrinthRawProject {
    fn into_comp_project(self) -> CompProject {
        let mut all_cats = self.categories;
        for c in self.additional_categories {
            if !all_cats.contains(&c) {
                all_cats.push(c);
            }
        }

        let comp_type = parse_comp_type(&self.project_type, &all_cats);
        let website_url = format!("https://modrinth.com/{}/{}", self.project_type, self.slug);
        let (zh_name, wiki_id) = if comp_type == CompType::Mod {
            match crate::modname::lookup_by_slug(&self.slug)
                .or_else(|| crate::modname::lookup_by_slug(&self.id))
            {
                Some(entry) => (
                    Some(crate::modname::display_zh(entry).to_string()),
                    Some(entry.wiki_id),
                ),
                None => (None, None),
            }
        } else {
            (None, None)
        };

        let mut loaders = self.loaders;
        let mut categories = Vec::new();
        for cat in all_cats {
            if is_loader(&cat) {
                if !loaders.contains(&cat) {
                    loaders.push(cat);
                }
            } else if !categories.contains(&cat) {
                categories.push(cat);
            }
        }

        CompProject {
            platform: CompPlatform::Modrinth,
            id: self.id,
            slug: self.slug,
            title: self.title,
            summary: self.description,
            description: self.body,
            icon_url: self.icon_url,
            author: self.team.unwrap_or_default(),
            downloads: self.downloads,
            follows: self.followers,
            categories,
            mc_versions: self.game_versions,
            loaders,
            comp_type,
            updated_at: self.updated,
            website_url: Some(website_url),
            zh_name,
            wiki_id,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ModrinthRawVersion {
    id: String,
    project_id: String,
    name: String,
    #[serde(default)]
    version_number: String,
    game_versions: Vec<String>,
    version_type: String,
    loaders: Vec<String>,
    date_published: String,
    #[serde(default)]
    files: Vec<ModrinthRawFile>,
    #[serde(default)]
    dependencies: Vec<ModrinthRawDependency>,
}

#[derive(Debug, Deserialize)]
struct ModrinthRawFile {
    #[serde(default)]
    hashes: ModrinthRawHashes,
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    size: u64,
}

#[derive(Debug, Default, Deserialize)]
struct ModrinthRawHashes {
    sha1: Option<String>,
    sha512: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthRawDependency {
    project_id: Option<String>,
    dependency_type: String,
}

impl ModrinthRawVersion {
    fn into_comp_file(self, priority: SourcePriority) -> Option<CompFile> {
        // Find primary file or first file
        let primary_file = self
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| self.files.first())?;

        let channel = match self.version_type.to_lowercase().as_str() {
            "beta" => CompReleaseChannel::Beta,
            "alpha" => CompReleaseChannel::Alpha,
            _ => CompReleaseChannel::Release,
        };

        // Download URLs rewritten via mclr_net
        let mut download_urls = Vec::new();
        if let Ok(parsed_url) = Url::parse(&primary_file.url) {
            let rewritten = rewrite_url(&parsed_url, SourceKind::ModCdn);
            let ladder = racing_ladder(priority);
            for (ladder_origin, _) in ladder {
                for (origin, candidate_url) in &rewritten {
                    let url_str = candidate_url.to_string();
                    if *origin == ladder_origin && !download_urls.contains(&url_str) {
                        download_urls.push(url_str);
                    }
                }
            }
            for (_, candidate_url) in &rewritten {
                let url_str = candidate_url.to_string();
                if !download_urls.contains(&url_str) {
                    download_urls.push(url_str);
                }
            }
            if download_urls.is_empty() {
                download_urls.push(primary_file.url.clone());
            }
        } else {
            download_urls.push(primary_file.url.clone());
        }

        let hashes = CompFileHashes {
            sha1: primary_file.hashes.sha1.clone(),
            sha512: primary_file.hashes.sha512.clone(),
            murmur2: None,
        };

        let dependencies = self
            .dependencies
            .into_iter()
            .filter_map(|d| {
                let project_id = d.project_id?;
                let kind = match d.dependency_type.to_lowercase().as_str() {
                    "required" => CompDependencyKind::Required,
                    "optional" => CompDependencyKind::Optional,
                    "incompatible" => CompDependencyKind::Incompatible,
                    "embedded" => CompDependencyKind::Embedded,
                    _ => CompDependencyKind::Optional,
                };
                Some(CompDependency { project_id, kind })
            })
            .collect();

        Some(CompFile {
            id: self.id,
            project_id: self.project_id,
            platform: CompPlatform::Modrinth,
            name: if self.name.is_empty() {
                self.version_number
            } else {
                self.name
            },
            file_name: primary_file.filename.clone(),
            mc_versions: self.game_versions,
            loaders: self.loaders,
            channel,
            release_date: self.date_published,
            size: primary_file.size,
            hashes,
            download_urls,
            distribution_disabled: false,
            dependencies,
        })
    }
}

fn parse_comp_type(project_type: &str, categories: &[String]) -> CompType {
    match project_type.to_lowercase().as_str() {
        "modpack" => CompType::Modpack,
        "resourcepack" => CompType::Resourcepack,
        "shader" => CompType::Shader,
        "datapack" => CompType::Datapack,
        _ => {
            // Check if categories explicitly contains datapack
            if categories
                .iter()
                .any(|c| c.eq_ignore_ascii_case("datapack"))
            {
                CompType::Datapack
            } else {
                CompType::Mod
            }
        }
    }
}

fn is_loader(cat: &str) -> bool {
    matches!(
        cat.to_ascii_lowercase().as_str(),
        "fabric"
            | "forge"
            | "neoforge"
            | "quilt"
            | "liteloader"
            | "modloader"
            | "rift"
            | "cleanroom"
            | "canvas"
            | "iris"
            | "optifine"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_search_mapping_and_facets() {
        let server = MockServer::start().await;

        let search_json = serde_json::json!({
            "hits": [
                {
                    "project_id": "AANobbMI",
                    "slug": "sodium",
                    "title": "Sodium",
                    "description": "A modern rendering engine for Minecraft",
                    "categories": ["fabric", "optimization"],
                    "client_side": "required",
                    "server_side": "unsupported",
                    "project_type": "mod",
                    "downloads": 25000000,
                    "icon_url": "https://cdn.modrinth.com/data/AANobbMI/icon.png",
                    "author": "jellysquid3",
                    "versions": ["1.20.1", "1.20.2"],
                    "follows": 45000,
                    "date_modified": "2023-10-01T12:00:00Z"
                }
            ],
            "offset": 0,
            "limit": 40,
            "total_hits": 1
        });

        Mock::given(method("GET"))
            .and(path("/v2/search"))
            .and(query_param("limit", "40"))
            .and(query_param("offset", "0"))
            .and(query_param("index", "downloads"))
            .and(query_param("query", "sodium"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&search_json))
            .mount(&server)
            .await;

        let client = ModrinthClient::default().with_base_url(server.uri());

        let q = CompSearchQuery {
            platform: CompPlatform::Modrinth,
            comp_type: CompType::Mod,
            query: "sodium".to_string(),
            mc_version: Some("1.20.1".to_string()),
            loader: Some("fabric".to_string()),
            category: Some("optimization".to_string()),
            sort: CompSort::Downloads,
            offset: 0,
            limit: 40,
        };

        let result = client.search(&q).await.expect("search should succeed");
        assert_eq!(result.total_hits, 1);
        assert_eq!(result.hits.len(), 1);

        let hit = &result.hits[0];
        assert_eq!(hit.id, "AANobbMI");
        assert_eq!(hit.slug, "sodium");
        assert_eq!(hit.title, "Sodium");
        assert_eq!(hit.comp_type, CompType::Mod);
        assert_eq!(hit.author, "jellysquid3");
        assert_eq!(hit.downloads, 25000000);
        assert_eq!(hit.follows, 45000);
        assert!(hit.loaders.contains(&"fabric".to_string()));
        assert!(hit.categories.contains(&"optimization".to_string()));
        assert_eq!(
            hit.website_url.as_deref(),
            Some("https://modrinth.com/mod/sodium")
        );
        assert_eq!(hit.zh_name.as_deref(), Some("钠"));
        assert_eq!(hit.wiki_id, Some(2785));
    }

    #[tokio::test]
    async fn test_get_project() {
        let server = MockServer::start().await;

        let project_json = serde_json::json!({
            "id": "AANobbMI",
            "slug": "sodium",
            "title": "Sodium",
            "description": "Summary of sodium",
            "body": "# Full Sodium Description\nMarkdown here.",
            "categories": ["fabric"],
            "additional_categories": ["optimization"],
            "project_type": "mod",
            "downloads": 12345,
            "icon_url": "https://cdn.modrinth.com/icon.png",
            "team": "CaffeineMC",
            "game_versions": ["1.20.1"],
            "loaders": ["fabric"],
            "followers": 789,
            "updated": "2023-11-01T00:00:00Z"
        });

        Mock::given(method("GET"))
            .and(path("/v2/project/sodium"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&project_json))
            .mount(&server)
            .await;

        let client = ModrinthClient::default().with_base_url(server.uri());
        let proj = client.get_project("sodium").await.expect("get_project");

        assert_eq!(proj.id, "AANobbMI");
        assert_eq!(proj.slug, "sodium");
        assert_eq!(
            proj.description.as_deref(),
            Some("# Full Sodium Description\nMarkdown here.")
        );
        assert_eq!(proj.author, "CaffeineMC");
        assert_eq!(
            proj.website_url.as_deref(),
            Some("https://modrinth.com/mod/sodium")
        );
    }

    #[tokio::test]
    async fn test_get_versions() {
        let server = MockServer::start().await;

        let versions_json = serde_json::json!([
            {
                "id": "ver123",
                "project_id": "AANobbMI",
                "name": "Sodium 0.5.8",
                "version_number": "0.5.8",
                "game_versions": ["1.20.1"],
                "version_type": "release",
                "loaders": ["fabric"],
                "date_published": "2023-12-01T00:00:00Z",
                "files": [
                    {
                        "hashes": {
                            "sha1": "aabbccddee112233",
                            "sha512": "5566778899aabbcc"
                        },
                        "url": "https://cdn.modrinth.com/data/AANobbMI/versions/ver123/sodium-0.5.8.jar",
                        "filename": "sodium-0.5.8.jar",
                        "primary": true,
                        "size": 1234567
                    }
                ],
                "dependencies": [
                    {
                        "project_id": "P7dR8mSH",
                        "dependency_type": "required"
                    },
                    {
                        "project_id": "opt123",
                        "dependency_type": "optional"
                    }
                ]
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/v2/project/AANobbMI/version"))
            .and(query_param("include_changelog", "false"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&versions_json))
            .mount(&server)
            .await;

        let client = ModrinthClient::default().with_base_url(server.uri());
        let versions = client
            .get_versions("AANobbMI", None, None)
            .await
            .expect("get_versions");

        assert_eq!(versions.len(), 1);
        let ver = &versions[0];
        assert_eq!(ver.id, "ver123");
        assert_eq!(ver.project_id, "AANobbMI");
        assert_eq!(ver.name, "Sodium 0.5.8");
        assert_eq!(ver.file_name, "sodium-0.5.8.jar");
        assert_eq!(ver.channel, CompReleaseChannel::Release);
        assert_eq!(ver.hashes.sha1.as_deref(), Some("aabbccddee112233"));
        assert_eq!(ver.hashes.sha512.as_deref(), Some("5566778899aabbcc"));
        assert_eq!(ver.size, 1234567);
        assert_eq!(ver.dependencies.len(), 2);
        assert_eq!(ver.dependencies[0].project_id, "P7dR8mSH");
        assert_eq!(ver.dependencies[0].kind, CompDependencyKind::Required);
        assert_eq!(ver.dependencies[1].project_id, "opt123");
        assert_eq!(ver.dependencies[1].kind, CompDependencyKind::Optional);

        // Download URLs rewritten with mirror
        assert!(!ver.download_urls.is_empty());
        assert!(
            ver.download_urls
                .iter()
                .any(|u| u.contains("mod.mcimirror.top"))
        );
    }

    #[tokio::test]
    async fn test_get_version_files_by_sha1() {
        let server = MockServer::start().await;

        let version_files_json = serde_json::json!({
            "aabbccddee": {
                "id": "v1",
                "project_id": "p1",
                "name": "Version 1",
                "version_number": "1.0.0",
                "game_versions": ["1.20.1"],
                "version_type": "release",
                "loaders": ["fabric"],
                "date_published": "2023-01-01T00:00:00Z",
                "files": [
                    {
                        "hashes": {
                            "sha1": "aabbccddee"
                        },
                        "url": "https://cdn.modrinth.com/v1.jar",
                        "filename": "v1.jar",
                        "primary": true,
                        "size": 1000
                    }
                ],
                "dependencies": []
            }
        });

        Mock::given(method("POST"))
            .and(path("/v2/version_files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&version_files_json))
            .mount(&server)
            .await;

        let client = ModrinthClient::default().with_base_url(server.uri());
        let res = client
            .get_version_files_by_sha1(&["aabbccddee".to_string()])
            .await
            .expect("get_version_files_by_sha1");

        assert_eq!(res.len(), 1);
        assert!(res.contains_key("aabbccddee"));
        assert_eq!(res["aabbccddee"].id, "v1");
        assert_eq!(res["aabbccddee"].file_name, "v1.jar");
    }

    #[tokio::test]
    async fn test_get_version_files_update() {
        let server = MockServer::start().await;

        let update_json = serde_json::json!({
            "aabb": {
                "id": "v2",
                "project_id": "p1",
                "name": "Version 2",
                "version_number": "2.0.0",
                "game_versions": ["1.20.1"],
                "version_type": "release",
                "loaders": ["fabric"],
                "date_published": "2023-02-01T00:00:00Z",
                "files": [
                    {
                        "hashes": {
                            "sha1": "newhash"
                        },
                        "url": "https://cdn.modrinth.com/v2.jar",
                        "filename": "v2.jar",
                        "primary": true,
                        "size": 2000
                    }
                ],
                "dependencies": []
            }
        });

        Mock::given(method("POST"))
            .and(path("/v2/version_files/update"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&update_json))
            .mount(&server)
            .await;

        let client = ModrinthClient::default().with_base_url(server.uri());
        let res = client
            .get_version_files_update(
                &["aabb".to_string()],
                &["fabric".to_string()],
                &["1.20.1".to_string()],
            )
            .await
            .expect("get_version_files_update");

        assert_eq!(res.len(), 1);
        assert_eq!(res["aabb"].id, "v2");
        assert_eq!(res["aabb"].file_name, "v2.jar");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_modrinth_search_sodium() {
        let client = ModrinthClient::default();
        let q = CompSearchQuery {
            platform: CompPlatform::Modrinth,
            comp_type: CompType::Mod,
            query: "sodium".to_string(),
            mc_version: Some("1.20.1".to_string()),
            loader: Some("fabric".to_string()),
            category: None,
            sort: CompSort::Relevance,
            offset: 0,
            limit: 10,
        };

        let res = client.search(&q).await.expect("live modrinth search");
        assert!(!res.hits.is_empty(), "expected hits from live search");
        let has_sodium = res.hits.iter().any(|h| h.slug == "sodium");
        assert!(has_sodium, "expected to find sodium in live search results");
    }
}
