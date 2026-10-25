//! Minecraft server list (`servers.dat`) CRUD, SRV resolution, Happy Eyeballs, and SLP ping.

use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

use crate::error::SavesError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub index: usize,
    pub name: String,
    pub ip: String,
    pub icon: Option<String>,
    pub accept_textures: Option<i8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerPingResult {
    pub ip: String,
    pub resolved_addr: String,
    pub online: bool,
    pub motd: String,
    pub version_name: String,
    pub protocol: i32,
    pub players_online: i32,
    pub players_max: i32,
    pub favicon: Option<String>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedServerAddress {
    pub host: String,
    pub ip: Option<String>,
    pub port: u16,
}

// ---------------------------------------------------------------------------
// NBT Data Structures for servers.dat
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ServersDatFile {
    #[serde(default)]
    servers: Vec<NbtServerEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
struct NbtServerEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    ip: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    icon: Option<String>,
    #[serde(
        rename = "acceptTextures",
        skip_serializing_if = "Option::is_none",
        default
    )]
    accept_textures: Option<i8>,
}

// ---------------------------------------------------------------------------
// servers.dat NBT CRUD
// ---------------------------------------------------------------------------

/// Reads servers from `servers.dat`. Returns an empty vector if the file does not exist.
pub fn read_servers_dat(path: &Path) -> Result<Vec<ServerEntry>, SavesError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    let decompressed = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        buf
    } else {
        bytes
    };

    let file: ServersDatFile = fastnbt::from_bytes(&decompressed)?;
    Ok(file
        .servers
        .into_iter()
        .enumerate()
        .map(|(index, entry)| ServerEntry {
            index,
            name: entry.name,
            ip: entry.ip,
            icon: entry.icon,
            accept_textures: entry.accept_textures,
        })
        .collect())
}

/// Atomically writes servers to `servers.dat` as uncompressed NBT.
pub fn write_servers_dat(path: &Path, entries: &[ServerEntry]) -> Result<(), SavesError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let servers = entries
        .iter()
        .map(|e| NbtServerEntry {
            name: e.name.clone(),
            ip: e.ip.clone(),
            icon: e.icon.clone(),
            accept_textures: e.accept_textures,
        })
        .collect();

    let file = ServersDatFile { servers };
    let bytes = fastnbt::to_bytes(&file)?;

    let tmp_path = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::write(&tmp_path, &bytes)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Adds a new server entry to `servers.dat` and returns the updated list.
pub fn add_server(
    path: &Path,
    name: String,
    ip: String,
    accept_textures: Option<i8>,
) -> Result<Vec<ServerEntry>, SavesError> {
    let mut entries = read_servers_dat(path)?;
    let index = entries.len();
    entries.push(ServerEntry {
        index,
        name,
        ip,
        icon: None,
        accept_textures,
    });
    write_servers_dat(path, &entries)?;
    Ok(entries)
}

/// Updates an existing server entry in `servers.dat` and returns the updated list.
pub fn update_server(
    path: &Path,
    index: usize,
    name: String,
    ip: String,
    accept_textures: Option<i8>,
) -> Result<Vec<ServerEntry>, SavesError> {
    let mut entries = read_servers_dat(path)?;
    if index >= entries.len() {
        return Err(SavesError::InvalidPath(format!(
            "server index out of bounds: {index}"
        )));
    }
    entries[index].name = name;
    entries[index].ip = ip;
    entries[index].accept_textures = accept_textures;
    write_servers_dat(path, &entries)?;
    Ok(entries)
}

/// Removes a server entry by index from `servers.dat` and returns the updated list.
pub fn remove_server(path: &Path, index: usize) -> Result<Vec<ServerEntry>, SavesError> {
    let mut entries = read_servers_dat(path)?;
    if index >= entries.len() {
        return Err(SavesError::InvalidPath(format!(
            "server index out of bounds: {index}"
        )));
    }
    entries.remove(index);
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.index = i;
    }
    write_servers_dat(path, &entries)?;
    Ok(entries)
}

// ---------------------------------------------------------------------------
// SRV Resolution & Happy Eyeballs
// ---------------------------------------------------------------------------

pub(crate) trait SrvLookup: Send + Sync {
    async fn lookup_srv(&self, host: &str) -> Option<(String, u16)>;
}

pub struct DefaultSrvLookup {
    resolver: hickory_resolver::TokioAsyncResolver,
}

impl DefaultSrvLookup {
    pub fn new() -> Self {
        let resolver = hickory_resolver::TokioAsyncResolver::tokio_from_system_conf()
            .unwrap_or_else(|_| {
                hickory_resolver::TokioAsyncResolver::tokio(
                    hickory_resolver::config::ResolverConfig::default(),
                    hickory_resolver::config::ResolverOpts::default(),
                )
            });
        Self { resolver }
    }
}

impl Default for DefaultSrvLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl SrvLookup for DefaultSrvLookup {
    async fn lookup_srv(&self, host: &str) -> Option<(String, u16)> {
        let query = format!("_minecraft._tcp.{}.", host.trim_end_matches('.'));
        let res = self.resolver.srv_lookup(query).await.ok()?;
        let mut records: Vec<_> = res.into_iter().collect();
        if records.is_empty() {
            return None;
        }
        records.sort_by_key(|r| r.priority());
        let first = records.into_iter().next()?;
        let target = first.target().to_utf8();
        let clean_target = target.trim_end_matches('.').to_string();
        if clean_target.is_empty() || clean_target == "." {
            return None;
        }
        Some((clean_target, first.port()))
    }
}

pub fn parse_host_and_port(address: &str) -> Result<(String, Option<u16>), SavesError> {
    let mut s = address.trim();
    if let Some(idx) = s.find("://") {
        s = &s[idx + 3..];
    }
    while s.ends_with('/') {
        s = &s[..s.len() - 1];
    }
    if s.is_empty() {
        return Err(SavesError::InvalidPath("address is empty".to_string()));
    }

    if s.starts_with('[') {
        if let Some(close_bracket) = s.find(']') {
            let ip = &s[1..close_bracket];
            if s.len() > close_bracket + 1 {
                if s.as_bytes()[close_bracket + 1] == b':' {
                    let port_str = &s[close_bracket + 2..];
                    let port = port_str.parse::<u16>().map_err(|e| {
                        SavesError::InvalidPath(format!("invalid port {port_str}: {e}"))
                    })?;
                    return Ok((ip.to_string(), Some(port)));
                }
                return Err(SavesError::InvalidPath(format!("invalid ipv6 format: {s}")));
            }
            return Ok((ip.to_string(), None));
        }
        return Err(SavesError::InvalidPath(format!(
            "unclosed ipv6 bracket: {s}"
        )));
    }

    if s.parse::<IpAddr>().is_ok() {
        return Ok((s.to_string(), None));
    }

    let colon_count = s.chars().filter(|&c| c == ':').count();
    if colon_count == 1 {
        let parts: Vec<&str> = s.split(':').collect();
        let host = parts[0];
        let port_str = parts[1];
        let port = port_str
            .parse::<u16>()
            .map_err(|e| SavesError::InvalidPath(format!("invalid port {port_str}: {e}")))?;
        return Ok((host.to_string(), Some(port)));
    }

    Ok((s.to_string(), None))
}

pub async fn resolve_server_address(address: &str) -> Result<ResolvedServerAddress, SavesError> {
    let resolver = DefaultSrvLookup::default();
    resolve_server_address_with_resolver(address, &resolver).await
}

pub(crate) async fn resolve_server_address_with_resolver<S: SrvLookup + ?Sized>(
    address: &str,
    resolver: &S,
) -> Result<ResolvedServerAddress, SavesError> {
    let (host, explicit_port) = parse_host_and_port(address)?;

    // If no explicit port or port is default 25565, query SRV if host is not a raw IP
    let (target_host, target_port) = if (explicit_port.is_none() || explicit_port == Some(25565))
        && host.parse::<IpAddr>().is_err()
    {
        if let Some((srv_host, srv_port)) = resolver.lookup_srv(&host).await {
            (srv_host, srv_port)
        } else {
            (host.clone(), explicit_port.unwrap_or(25565))
        }
    } else {
        (host.clone(), explicit_port.unwrap_or(25565))
    };

    let ip = if let Ok(ip) = target_host.parse::<IpAddr>() {
        Some(ip.to_string())
    } else {
        tokio::net::lookup_host(format!("{target_host}:{target_port}"))
            .await
            .ok()
            .and_then(|mut addrs| addrs.next().map(|a| a.ip().to_string()))
    };

    Ok(ResolvedServerAddress {
        host: target_host,
        ip,
        port: target_port,
    })
}

/// Happy Eyeballs (RFC 8305, 250 ms stagger) TCP connection.
pub async fn happy_eyeballs_connect(
    candidates: &[SocketAddr],
    connect_timeout: Duration,
) -> Result<(TcpStream, SocketAddr), SavesError> {
    if candidates.is_empty() {
        return Err(SavesError::PingFailed("no address to connect".to_string()));
    }
    if candidates.len() == 1 {
        let addr = candidates[0];
        let stream = timeout(connect_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| SavesError::PingFailed(format!("connection timed out to {addr}")))?
            .map_err(|e| SavesError::PingFailed(format!("failed to connect to {addr}: {e}")))?;
        let _ = stream.set_nodelay(true);
        return Ok((stream, addr));
    }

    let (tx, mut rx) =
        mpsc::channel::<(Result<TcpStream, std::io::Error>, SocketAddr)>(candidates.len());
    let mut remaining = candidates.len();
    let mut launched = 0;

    let total_timeout = sleep(connect_timeout);
    tokio::pin!(total_timeout);

    let addr0 = candidates[0];
    let tx0 = tx.clone();
    tokio::spawn(async move {
        let res = TcpStream::connect(addr0).await;
        let _ = tx0.send((res, addr0)).await;
    });
    launched += 1;

    let mut stagger_timer = Box::pin(sleep(Duration::from_millis(250)));
    let mut last_error = None;

    loop {
        tokio::select! {
            _ = &mut total_timeout => {
                return Err(SavesError::PingFailed(format!(
                    "connection timed out after {connect_timeout:?}, last error: {last_error:?}"
                )));
            }
            _ = &mut stagger_timer, if launched < candidates.len() => {
                let next_addr = candidates[launched];
                let tx_next = tx.clone();
                tokio::spawn(async move {
                    let res = TcpStream::connect(next_addr).await;
                    let _ = tx_next.send((res, next_addr)).await;
                });
                launched += 1;
                if launched < candidates.len() {
                    stagger_timer = Box::pin(sleep(Duration::from_millis(250)));
                }
            }
            Some((res, addr)) = rx.recv() => {
                remaining -= 1;
                match res {
                    Ok(stream) => {
                        let _ = stream.set_nodelay(true);
                        return Ok((stream, addr));
                    }
                    Err(e) => {
                        last_error = Some(e);
                        if launched < candidates.len() {
                            let next_addr = candidates[launched];
                            let tx_next = tx.clone();
                            tokio::spawn(async move {
                                let res = TcpStream::connect(next_addr).await;
                                let _ = tx_next.send((res, next_addr)).await;
                            });
                            launched += 1;
                            if launched < candidates.len() {
                                stagger_timer = Box::pin(sleep(Duration::from_millis(250)));
                            }
                        }
                        if remaining == 0 {
                            return Err(SavesError::PingFailed(format!(
                                "all connection attempts failed, last error: {last_error:?}"
                            )));
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Minecraft Server List Ping (SLP)
// ---------------------------------------------------------------------------

fn encode_varint(mut val: i32, buf: &mut Vec<u8>) {
    loop {
        if (val & !0x7F) == 0 {
            buf.push(val as u8);
            return;
        }
        buf.push(((val & 0x7F) | 0x80) as u8);
        val = (val as u32 >> 7) as i32;
    }
}

async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> Result<i32, SavesError> {
    let mut num_read = 0;
    let mut result = 0i32;
    loop {
        let mut byte = [0u8; 1];
        reader
            .read_exact(&mut byte)
            .await
            .map_err(|e| SavesError::PingFailed(format!("failed to read varint: {e}")))?;
        let value = (byte[0] & 0x7F) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err(SavesError::PingFailed("VarInt is too big".to_string()));
        }
        if (byte[0] & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

fn decode_varint_slice(slice: &[u8]) -> Result<(i32, usize), SavesError> {
    let mut num_read = 0;
    let mut result = 0i32;
    for &b in slice {
        let value = (b & 0x7F) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err(SavesError::PingFailed("VarInt is too big".to_string()));
        }
        if (b & 0x80) == 0 {
            return Ok((result, num_read));
        }
    }
    Err(SavesError::PingFailed("VarInt truncated".to_string()))
}

#[derive(Debug, Deserialize)]
struct SlpResponse {
    #[serde(default)]
    version: Option<SlpVersion>,
    #[serde(default)]
    players: Option<SlpPlayers>,
    #[serde(default)]
    description: Option<serde_json::Value>,
    #[serde(default)]
    favicon: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SlpVersion {
    #[serde(default)]
    name: String,
    #[serde(default)]
    protocol: i32,
}

#[derive(Debug, Deserialize, Default)]
struct SlpPlayers {
    #[serde(default)]
    max: i32,
    #[serde(default)]
    online: i32,
}

fn parse_motd(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => {
            let mut result = String::new();
            if let Some(serde_json::Value::String(text)) = map.get("text") {
                result.push_str(text);
            }
            if let Some(serde_json::Value::Array(extra)) = map.get("extra") {
                for item in extra {
                    result.push_str(&parse_motd(item));
                }
            }
            result
        }
        serde_json::Value::Array(arr) => {
            let mut result = String::new();
            for item in arr {
                result.push_str(&parse_motd(item));
            }
            result
        }
        _ => String::new(),
    }
}

pub async fn ping_server(ip: &str) -> Result<ServerPingResult, SavesError> {
    let resolver = DefaultSrvLookup::default();
    ping_server_with_resolver(ip, &resolver).await
}

pub(crate) async fn ping_server_with_resolver<S: SrvLookup + ?Sized>(
    ip: &str,
    resolver: &S,
) -> Result<ServerPingResult, SavesError> {
    match ping_server_internal(ip, resolver).await {
        Ok(res) => Ok(res),
        Err(e) => {
            tracing::warn!("ping_server failed for {ip}: {e}");
            Ok(ServerPingResult {
                ip: ip.to_string(),
                resolved_addr: String::new(),
                online: false,
                motd: String::new(),
                version_name: String::new(),
                protocol: 0,
                players_online: 0,
                players_max: 0,
                favicon: None,
                latency_ms: 0,
            })
        }
    }
}

async fn ping_server_internal<S: SrvLookup + ?Sized>(
    ip: &str,
    resolver: &S,
) -> Result<ServerPingResult, SavesError> {
    let resolved = resolve_server_address_with_resolver(ip, resolver).await?;
    let target_host = &resolved.host;
    let target_port = resolved.port;

    let candidates: Vec<SocketAddr> = if let Ok(ip_addr) = target_host.parse::<IpAddr>() {
        vec![SocketAddr::new(ip_addr, target_port)]
    } else {
        let addrs = tokio::net::lookup_host(format!("{target_host}:{target_port}"))
            .await
            .map_err(|e| {
                SavesError::PingFailed(format!("failed to resolve host {target_host}: {e}"))
            })?;
        let (mut v6, mut v4): (Vec<SocketAddr>, Vec<SocketAddr>) = addrs.partition(|a| a.is_ipv6());
        let mut interleaved = Vec::new();
        while !v6.is_empty() || !v4.is_empty() {
            if !v6.is_empty() {
                interleaved.push(v6.remove(0));
            }
            if !v4.is_empty() {
                interleaved.push(v4.remove(0));
            }
        }
        interleaved
    };

    if candidates.is_empty() {
        return Err(SavesError::PingFailed(format!(
            "no addresses found for host {target_host}"
        )));
    }

    let (mut stream, socket_addr) =
        happy_eyeballs_connect(&candidates, Duration::from_secs(5)).await?;

    // 1. Send Handshake Packet (packet_id = 0x00, protocol = 763, next_state = 1)
    let mut packet_data = Vec::new();
    encode_varint(0x00, &mut packet_data);
    encode_varint(763, &mut packet_data);
    encode_varint(target_host.len() as i32, &mut packet_data);
    packet_data.extend_from_slice(target_host.as_bytes());
    packet_data.extend_from_slice(&target_port.to_be_bytes());
    encode_varint(1, &mut packet_data);

    let mut handshake = Vec::new();
    encode_varint(packet_data.len() as i32, &mut handshake);
    handshake.extend_from_slice(&packet_data);
    stream
        .write_all(&handshake)
        .await
        .map_err(|e| SavesError::PingFailed(format!("failed to write handshake: {e}")))?;

    // 2. Send Status Request Packet (length = 0x01, packet_id = 0x00)
    stream
        .write_all(&[0x01, 0x00])
        .await
        .map_err(|e| SavesError::PingFailed(format!("failed to write status request: {e}")))?;
    stream
        .flush()
        .await
        .map_err(|e| SavesError::PingFailed(format!("failed to flush: {e}")))?;

    // 3. Read Status Response Packet
    let packet_length = read_varint(&mut stream).await?;
    if packet_length <= 0 || packet_length > 1_048_576 {
        return Err(SavesError::PingFailed(format!(
            "invalid packet length: {packet_length}"
        )));
    }

    let mut packet_bytes = vec![0u8; packet_length as usize];
    stream
        .read_exact(&mut packet_bytes)
        .await
        .map_err(|e| SavesError::PingFailed(format!("failed to read status response: {e}")))?;

    let (packet_id, off1) = decode_varint_slice(&packet_bytes)?;
    if packet_id != 0x00 {
        return Err(SavesError::PingFailed(format!(
            "expected packet ID 0, got {packet_id}"
        )));
    }

    let (json_len, off2) = decode_varint_slice(&packet_bytes[off1..])?;
    let json_start = off1 + off2;
    let json_end = json_start + json_len as usize;
    if json_end > packet_bytes.len() {
        return Err(SavesError::PingFailed(
            "json payload truncated in status response".to_string(),
        ));
    }
    let json_str = std::str::from_utf8(&packet_bytes[json_start..json_end])
        .map_err(|e| SavesError::PingFailed(format!("invalid utf-8 in status response: {e}")))?;

    let slp: SlpResponse = serde_json::from_str(json_str)
        .map_err(|e| SavesError::PingFailed(format!("failed to parse slp json: {e}")))?;

    let motd = slp.description.as_ref().map(parse_motd).unwrap_or_default();
    let version_name = slp
        .version
        .as_ref()
        .map(|v| v.name.clone())
        .unwrap_or_default();
    let protocol = slp.version.as_ref().map(|v| v.protocol).unwrap_or(0);
    let players_online = slp.players.as_ref().map(|p| p.online).unwrap_or(0);
    let players_max = slp.players.as_ref().map(|p| p.max).unwrap_or(0);
    let favicon = slp.favicon;

    // 4. Send Ping Packet & measure RTT
    let ping_start = std::time::Instant::now();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let mut ping_packet = vec![0x09, 0x01];
    ping_packet.extend_from_slice(&timestamp.to_be_bytes());
    let _ = stream.write_all(&ping_packet).await;
    let _ = stream.flush().await;

    // Read Pong packet with 2s timeout
    let pong_res = timeout(Duration::from_secs(2), async {
        let pong_len = read_varint(&mut stream).await?;
        let mut pong_buf = vec![0u8; pong_len as usize];
        stream
            .read_exact(&mut pong_buf)
            .await
            .map_err(|e| SavesError::PingFailed(e.to_string()))?;
        let (pong_id, _) = decode_varint_slice(&pong_buf)?;
        if pong_id != 0x01 {
            return Err(SavesError::PingFailed(format!(
                "unexpected pong id: {pong_id}"
            )));
        }
        Ok::<(), SavesError>(())
    })
    .await;

    let latency_ms = match pong_res {
        Ok(Ok(())) => ping_start.elapsed().as_millis().max(1) as u32,
        _ => ping_start.elapsed().as_millis().max(1) as u32,
    };

    Ok(ServerPingResult {
        ip: ip.to_string(),
        resolved_addr: socket_addr.to_string(),
        online: true,
        motd,
        version_name,
        protocol,
        players_online,
        players_max,
        favicon,
        latency_ms,
    })
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    #[test]
    fn test_servers_dat_crud_round_trip() {
        let dir = tempdir().unwrap();
        let servers_path = dir.path().join("servers.dat");

        // 1. Initial read should be empty
        let initial = read_servers_dat(&servers_path).unwrap();
        assert!(initial.is_empty());

        // 2. Add first server
        let list1 = add_server(
            &servers_path,
            "Hypixel".to_string(),
            "mc.hypixel.net".to_string(),
            Some(1),
        )
        .unwrap();
        assert_eq!(list1.len(), 1);
        assert_eq!(list1[0].name, "Hypixel");
        assert_eq!(list1[0].ip, "mc.hypixel.net");
        assert_eq!(list1[0].accept_textures, Some(1));

        // 3. Add second server
        let list2 = add_server(
            &servers_path,
            "Mineplex".to_string(),
            "us.mineplex.com".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(list2.len(), 2);
        assert_eq!(list2[1].name, "Mineplex");
        assert_eq!(list2[1].index, 1);

        // 4. Update first server
        let list3 = update_server(
            &servers_path,
            0,
            "Hypixel Network".to_string(),
            "play.hypixel.net".to_string(),
            Some(0),
        )
        .unwrap();
        assert_eq!(list3.len(), 2);
        assert_eq!(list3[0].name, "Hypixel Network");
        assert_eq!(list3[0].ip, "play.hypixel.net");
        assert_eq!(list3[0].accept_textures, Some(0));

        // 5. Remove first server
        let list4 = remove_server(&servers_path, 0).unwrap();
        assert_eq!(list4.len(), 1);
        assert_eq!(list4[0].index, 0);
        assert_eq!(list4[0].name, "Mineplex");

        // 6. Verify read matches written state
        let read_back = read_servers_dat(&servers_path).unwrap();
        assert_eq!(read_back, list4);
    }

    struct MockSrvResolver;

    impl SrvLookup for MockSrvResolver {
        async fn lookup_srv(&self, host: &str) -> Option<(String, u16)> {
            if host == "play.example.com" {
                Some(("mc.backend.example.com".to_string(), 25577))
            } else {
                None
            }
        }
    }

    #[tokio::test]
    async fn test_srv_resolution_with_injected_resolver() {
        let resolver = MockSrvResolver;

        // No explicit port -> queries SRV
        let res1 = resolve_server_address_with_resolver("play.example.com", &resolver)
            .await
            .unwrap();
        assert_eq!(res1.host, "mc.backend.example.com");
        assert_eq!(res1.port, 25577);

        // Default port 25565 -> queries SRV
        let res2 = resolve_server_address_with_resolver("play.example.com:25565", &resolver)
            .await
            .unwrap();
        assert_eq!(res2.host, "mc.backend.example.com");
        assert_eq!(res2.port, 25577);

        // Explicit non-25565 port -> skips SRV
        let res3 = resolve_server_address_with_resolver("play.example.com:25566", &resolver)
            .await
            .unwrap();
        assert_eq!(res3.host, "play.example.com");
        assert_eq!(res3.port, 25566);
    }

    #[tokio::test]
    async fn test_slp_ping_against_local_fake_tcp_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // 1. Read handshake
            let hs_len = read_varint(&mut stream).await.unwrap();
            let mut hs_buf = vec![0u8; hs_len as usize];
            stream.read_exact(&mut hs_buf).await.unwrap();
            let (hs_id, _) = decode_varint_slice(&hs_buf).unwrap();
            assert_eq!(hs_id, 0x00);

            // 2. Read status request
            let req_len = read_varint(&mut stream).await.unwrap();
            assert_eq!(req_len, 0x01);
            let mut req_buf = vec![0u8; 1];
            stream.read_exact(&mut req_buf).await.unwrap();
            assert_eq!(req_buf[0], 0x00);

            // 3. Send status response
            let status_json = serde_json::json!({
                "version": {
                    "name": "1.20.1",
                    "protocol": 763
                },
                "players": {
                    "max": 20,
                    "online": 5
                },
                "description": "Hello MCLR"
            })
            .to_string();

            let json_bytes = status_json.as_bytes();
            let mut inner = Vec::new();
            encode_varint(0x00, &mut inner);
            encode_varint(json_bytes.len() as i32, &mut inner);
            inner.extend_from_slice(json_bytes);

            let mut resp = Vec::new();
            encode_varint(inner.len() as i32, &mut resp);
            resp.extend_from_slice(&inner);

            stream.write_all(&resp).await.unwrap();
            stream.flush().await.unwrap();

            // 4. Read ping packet
            let ping_len = read_varint(&mut stream).await.unwrap();
            assert_eq!(ping_len, 9);
            let mut ping_buf = vec![0u8; 9];
            stream.read_exact(&mut ping_buf).await.unwrap();
            assert_eq!(ping_buf[0], 0x01);
            let timestamp_payload = &ping_buf[1..9];

            // 5. Echo pong packet
            let mut pong = vec![0x09, 0x01];
            pong.extend_from_slice(timestamp_payload);
            stream.write_all(&pong).await.unwrap();
            stream.flush().await.unwrap();
        });

        let target_addr = format!("127.0.0.1:{}", addr.port());
        let result = ping_server(&target_addr).await.unwrap();

        assert!(result.online);
        assert_eq!(result.motd, "Hello MCLR");
        assert_eq!(result.players_online, 5);
        assert_eq!(result.players_max, 20);
        assert_eq!(result.version_name, "1.20.1");
        assert_eq!(result.protocol, 763);
        assert!(result.latency_ms > 0);

        server_task.await.unwrap();
    }
}
