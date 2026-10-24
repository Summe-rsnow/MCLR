use crate::error::NetError;
use mclr_core::SourcePriority;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::debug;
use url::Url;

/// Classification of URLs for appropriate mirror replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Assets,
    VersionMeta,
    Libraries,
    ModrinthApi,
    CurseForgeApi,
    ModCdn,
    Generic,
}

/// Source origin identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SourceOrigin {
    Official,
    Mirror,
}

/// Rewrites a URL based on its SourceKind into official and mirror URL variants.
/// Returns a list of `(SourceOrigin, Url)` pairs.
pub fn rewrite_url(url: &Url, kind: SourceKind) -> Vec<(SourceOrigin, Url)> {
    let host = match url.host_str() {
        Some(h) => h,
        None => return vec![(SourceOrigin::Official, url.clone())],
    };

    let mut results = Vec::new();
    let original = (SourceOrigin::Official, url.clone());

    match kind {
        SourceKind::Assets => {
            // resources.download.minecraft.net -> bmclapi2.bangbang93.com/assets
            // piston-data.mojang.com / piston-meta.mojang.com -> bmclapi2.bangbang93.com/assets
            if host.eq_ignore_ascii_case("resources.download.minecraft.net")
                || host.eq_ignore_ascii_case("piston-data.mojang.com")
                || host.eq_ignore_ascii_case("piston-meta.mojang.com")
            {
                let path = url.path();
                let mirror_str = format!("https://bmclapi2.bangbang93.com/assets{path}");
                if let Ok(mirror_url) = Url::parse(&mirror_str) {
                    results.push((SourceOrigin::Mirror, mirror_url));
                }
            }
            results.push(original);
        }
        SourceKind::VersionMeta => {
            // piston-meta.mojang.com / launchermeta.mojang.com / launcher.mojang.com -> bmclapi2.bangbang93.com
            if host.eq_ignore_ascii_case("piston-meta.mojang.com")
                || host.eq_ignore_ascii_case("launchermeta.mojang.com")
                || host.eq_ignore_ascii_case("launcher.mojang.com")
            {
                let path_and_query = match url.query() {
                    Some(q) => format!("{}?{}", url.path(), q),
                    None => url.path().to_string(),
                };
                let mirror_str = format!("https://bmclapi2.bangbang93.com{path_and_query}");
                if let Ok(mirror_url) = Url::parse(&mirror_str) {
                    results.push((SourceOrigin::Mirror, mirror_url));
                }
            }
            results.push(original);
        }
        SourceKind::Libraries => {
            // libraries.minecraft.net / piston-data.mojang.com -> bmclapi2.bangbang93.com/maven
            if host.eq_ignore_ascii_case("libraries.minecraft.net")
                || host.eq_ignore_ascii_case("piston-data.mojang.com")
                || host.eq_ignore_ascii_case("piston-meta.mojang.com")
            {
                let path = url.path();
                let mirror_str = format!("https://bmclapi2.bangbang93.com/maven{path}");
                if let Ok(mirror_url) = Url::parse(&mirror_str) {
                    results.push((SourceOrigin::Mirror, mirror_url));
                }
            }
            results.push(original);
        }
        SourceKind::ModrinthApi => {
            // api.modrinth.com -> mod.mcimirror.top/modrinth
            if host.eq_ignore_ascii_case("api.modrinth.com") {
                let path_and_query = match url.query() {
                    Some(q) => format!("{}?{}", url.path(), q),
                    None => url.path().to_string(),
                };
                let mirror_str = format!("https://mod.mcimirror.top/modrinth{path_and_query}");
                if let Ok(mirror_url) = Url::parse(&mirror_str) {
                    results.push((SourceOrigin::Mirror, mirror_url));
                }
            }
            results.push(original);
        }
        SourceKind::CurseForgeApi => {
            // api.curseforge.com -> mod.mcimirror.top/curseforge
            if host.eq_ignore_ascii_case("api.curseforge.com") {
                let path_and_query = match url.query() {
                    Some(q) => format!("{}?{}", url.path(), q),
                    None => url.path().to_string(),
                };
                let mirror_str = format!("https://mod.mcimirror.top/curseforge{path_and_query}");
                if let Ok(mirror_url) = Url::parse(&mirror_str) {
                    results.push((SourceOrigin::Mirror, mirror_url));
                }
            }
            results.push(original);
        }
        SourceKind::ModCdn => {
            // cdn.modrinth.com / edge.forgecdn.net / mediafilez.forgecdn.net -> mod.mcimirror.top
            if host.eq_ignore_ascii_case("cdn.modrinth.com")
                || host.eq_ignore_ascii_case("edge.forgecdn.net")
                || host.eq_ignore_ascii_case("mediafilez.forgecdn.net")
            {
                let path_and_query = match url.query() {
                    Some(q) => format!("{}?{}", url.path(), q),
                    None => url.path().to_string(),
                };
                let mirror_str = format!("https://mod.mcimirror.top{path_and_query}");
                if let Ok(mirror_url) = Url::parse(&mirror_str) {
                    results.push((SourceOrigin::Mirror, mirror_url));
                }
            }
            results.push(original);
        }
        SourceKind::Generic => {
            results.push(original);
        }
    }

    results
}

/// Generates racing ladder timetable `(SourceOrigin, start_delay)` based on priority.
pub fn racing_ladder(priority: SourcePriority) -> Vec<(SourceOrigin, Duration)> {
    match priority {
        SourcePriority::MirrorFirst => vec![
            (SourceOrigin::Mirror, Duration::from_secs(0)),
            (SourceOrigin::Official, Duration::from_secs(30)),
        ],
        SourcePriority::Smart => vec![
            (SourceOrigin::Official, Duration::from_secs(0)),
            (SourceOrigin::Mirror, Duration::from_secs(5)),
        ],
        SourcePriority::OfficialFirst => vec![
            (SourceOrigin::Official, Duration::from_secs(0)),
            (SourceOrigin::Mirror, Duration::from_secs(60)),
        ],
    }
}

/// Races to fetch bytes from candidate sources based on rewritten URLs and the ladder schedule.
/// A later source starts when its delay elapses OR all earlier started sources have failed.
/// The first source to return 200 OK and successfully read bytes cancels all others.
pub async fn race_get_bytes(
    client: &Client,
    url: &Url,
    kind: SourceKind,
    priority: SourcePriority,
) -> Result<Vec<u8>, NetError> {
    let variants = rewrite_url(url, kind);
    if variants.len() == 1 {
        // Only one variant available (no mirror), fetch directly
        let res = client
            .get(variants[0].1.clone())
            .send()
            .await
            .map_err(NetError::Http)?;
        if !res.status().is_success() {
            return Err(NetError::Http(res.error_for_status().unwrap_err()));
        }
        let bytes = res.bytes().await.map_err(NetError::Http)?;
        return Ok(bytes.to_vec());
    }

    let ladder = racing_ladder(priority);

    // Build the ordered candidates matching the ladder
    let mut candidates: Vec<(Url, Duration)> = Vec::new();
    for (ladder_origin, delay) in ladder {
        if let Some((_, candidate_url)) = variants.iter().find(|(orig, _)| *orig == ladder_origin) {
            candidates.push((candidate_url.clone(), delay));
        }
    }
    // Also append any variant not included in ladder
    for (_, candidate_url) in &variants {
        if !candidates.iter().any(|(u, _)| u == candidate_url) {
            candidates.push((candidate_url.clone(), Duration::from_secs(60)));
        }
    }

    race_candidates(client, candidates).await
}

pub async fn race_candidates(
    client: &Client,
    candidates: Vec<(Url, Duration)>,
) -> Result<Vec<u8>, NetError> {
    if candidates.is_empty() {
        return Err(NetError::SourcesExhausted("No candidates to race".into()));
    }

    let (success_tx, mut success_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1);
    let (fail_notify_tx, mut fail_notify_rx) =
        tokio::sync::mpsc::channel::<usize>(candidates.len());
    let mut started_flags = vec![false; candidates.len()];
    let mut failed_flags = vec![false; candidates.len()];
    let mut cancel_senders: Vec<oneshot::Sender<()>> = Vec::new();

    let start_candidate = |index: usize,
                           url: Url,
                           client: Client,
                           success_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
                           fail_notify_tx: tokio::sync::mpsc::Sender<usize>|
     -> oneshot::Sender<()> {
        let (cancel_tx, cancel_rx) = oneshot::channel();
        tokio::spawn(async move {
            tokio::select! {
                _ = cancel_rx => {
                    // Canceled
                }
                result = async {
                    let resp = client.get(url.clone()).send().await?;
                    if !resp.status().is_success() {
                        return Err(NetError::Http(resp.error_for_status().unwrap_err()));
                    }
                    let bytes = resp.bytes().await.map_err(NetError::Http)?;
                    Ok(bytes.to_vec())
                } => {
                    match result {
                        Ok(data) => {
                            let _ = success_tx.send(data).await;
                        }
                        Err(e) => {
                            debug!("Racing candidate {} failed: {e}", url);
                            let _ = fail_notify_tx.send(index).await;
                        }
                    }
                }
            }
        });
        cancel_tx
    };

    // Start first candidate immediately
    started_flags[0] = true;
    let cancel = start_candidate(
        0,
        candidates[0].0.clone(),
        client.clone(),
        success_tx.clone(),
        fail_notify_tx.clone(),
    );
    cancel_senders.push(cancel);

    let start_time = tokio::time::Instant::now();

    loop {
        // Determine earliest next candidate to start based on duration
        let mut next_timer = None;
        let mut next_to_start = None;

        for (idx, (_, delay)) in candidates.iter().enumerate() {
            if !started_flags[idx] {
                let scheduled = start_time + *delay;
                next_timer = Some(scheduled);
                next_to_start = Some(idx);
                break;
            }
        }

        tokio::select! {
            Some(data) = success_rx.recv() => {
                // We have a winner! Cancel others
                for cancel in cancel_senders {
                    let _ = cancel.send(());
                }
                return Ok(data);
            }
            Some(failed_idx) = fail_notify_rx.recv() => {
                failed_flags[failed_idx] = true;

                // Check if all started candidates have failed
                let all_started_failed = started_flags
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| **s)
                    .all(|(i, _)| failed_flags[i]);

                if all_started_failed {
                    // Trigger the next unstarted candidate immediately
                    if let Some(next_idx) = started_flags.iter().position(|&s| !s) {
                        started_flags[next_idx] = true;
                        let cancel = start_candidate(
                            next_idx,
                            candidates[next_idx].0.clone(),
                            client.clone(),
                            success_tx.clone(),
                            fail_notify_tx.clone(),
                        );
                        cancel_senders.push(cancel);
                    } else {
                        // All candidates started and all failed
                        return Err(NetError::RacingFailed("All racing candidates failed".to_string()));
                    }
                }
            }
            _ = async {
                if let Some(target) = next_timer {
                    tokio::time::sleep_until(target).await;
                } else {
                    futures_util::future::pending::<()>().await;
                }
            } => {
                if let Some(next_idx) = next_to_start
                    && !started_flags[next_idx]
                {
                    started_flags[next_idx] = true;
                    let cancel = start_candidate(
                        next_idx,
                        candidates[next_idx].0.clone(),
                        client.clone(),
                        success_tx.clone(),
                        fail_notify_tx.clone(),
                    );
                    cancel_senders.push(cancel);
                }
            }
        }
    }
}
