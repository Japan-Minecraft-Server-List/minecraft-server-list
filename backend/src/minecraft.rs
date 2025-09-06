use anyhow::{Context, Result};
use core::fmt;
use serde::Deserialize;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::timeout;

use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::proto::rr::rdata::SRV;

#[derive(Debug, Clone)]
pub struct MinecraftServerInfo {
    pub host: String,
    pub port_effective: u16,  // 実際に使われたポート（SRV あり/なし）
    pub resolved: SocketAddr, // 接続に使われたIP:port
    pub connect_ms: u128,
    pub rtt_ms: u128,
    pub version_name: String,
    pub version_protocol: i32,
    pub players_online: i32,
    pub players_max: i32,
    pub motd: String,
}

impl MinecraftServerInfo {
    /// Handshake → Status → Ping を実行
    /// `port`: Some(..) なら SRV をスキップ、None なら SRV を試す（失敗時は 25565）。
    pub async fn query(host: &str, port: Option<u16>) -> Result<Self> {
        let force_v4 = std::env::var("MC_FORCE_IPV4").ok().as_deref() == Some("1");
        let resolver = TokioAsyncResolver::tokio_from_system_conf().context("init resolver")?;

        // 1) SRV を考慮した接続候補（SocketAddr）を構築
        let candidates = resolve_minecraft_candidates(&resolver, host, port, force_v4)
            .await
            .with_context(|| format!("DNS/SRV resolution failed for {host}"))?;
        if candidates.is_empty() {
            anyhow::bail!("no candidates to connect");
        }

        // 2) 候補すべてに同時接続（最初に成功したものを採用）
        let per_attempt = Duration::from_secs(3);
        let connect_start = Instant::now();
        let (mut stream, chosen_addr) = connect_race_all(&candidates, per_attempt)
            .await
            .with_context(|| {
                format!(
                    "TCP connect failed (tried concurrently: {})",
                    join_addrs(&candidates)
                )
            })?;
        let connect_ms = connect_start.elapsed().as_millis();

        // 以降の I/O のソフトタイムアウト
        let op_timeout = Duration::from_secs(5);

        // 3) Handshake（next state = 1: status）
        // server address には **元のホスト名（ユーザー入力）**を入れる（Bungee 等のため）
        // port は **実際に接続したポート**（SRV の結果を含む）
        let protocol_version = 47; // status では任意。互換性重視
        let handshake_port = chosen_addr.port();

        let mut payload = Vec::new();
        payload.push(0x00); // packet id
        write_varint(protocol_version, &mut payload);
        write_mc_string(host, &mut payload);
        payload.extend_from_slice(&handshake_port.to_be_bytes());
        write_varint(1, &mut payload); // next state = status

        let mut packet = Vec::new();
        write_varint(payload.len() as i32, &mut packet);
        packet.extend_from_slice(&payload);
        timeout(op_timeout, stream.write_all(&packet)).await??;

        // 4) Status Request
        timeout(op_timeout, stream.write_all(&[0x01, 0x00])).await??;

        // 5) Status Response（JSON）
        let _len = timeout(op_timeout, read_varint(&mut stream)).await??;
        let pid = timeout(op_timeout, read_varint(&mut stream)).await??;
        if pid != 0x00 {
            anyhow::bail!("Unexpected packet id (expected 0x00), got {pid}");
        }
        let json_len = timeout(op_timeout, read_varint(&mut stream)).await?? as usize;
        let json_bytes = timeout(op_timeout, read_exact_n(&mut stream, json_len)).await??;
        let json_text = String::from_utf8(json_bytes)?;
        let status: StatusResponse = serde_json::from_str(&json_text)?;

        // 6) Ping（往復遅延）
        let payload_time = 0_i64;
        let mut ping_payload = Vec::new();
        ping_payload.push(0x01);
        ping_payload.extend_from_slice(&payload_time.to_be_bytes());

        let mut ping_packet = Vec::new();
        write_varint(ping_payload.len() as i32, &mut ping_packet);
        ping_packet.extend_from_slice(&ping_payload);

        let ping_start = Instant::now();
        timeout(op_timeout, stream.write_all(&ping_packet)).await??;

        let _pong_len = timeout(op_timeout, read_varint(&mut stream)).await??;
        let pong_pid = timeout(op_timeout, read_varint(&mut stream)).await??;
        if pong_pid != 0x01 {
            anyhow::bail!("Unexpected pong packet id (expected 0x01), got {pong_pid}");
        }
        let mut pong_buf = [0u8; 8];
        timeout(op_timeout, stream.read_exact(&mut pong_buf)).await??;
        let _pong_value = i64::from_be_bytes(pong_buf);
        let rtt_ms = ping_start.elapsed().as_millis();

        // 7) 整形
        let motd = description_to_text(&status.description);
        Ok(Self {
            host: host.to_string(),
            port_effective: handshake_port,
            resolved: chosen_addr,
            connect_ms,
            rtt_ms,
            version_name: status.version.name,
            version_protocol: status.version.protocol,
            players_online: status.players.online,
            players_max: status.players.max,
            motd,
        })
    }
}

/* ---------- SRV 対応の候補生成 ---------- */

async fn resolve_minecraft_candidates(
    resolver: &TokioAsyncResolver,
    host: &str,
    port: Option<u16>,
    force_v4: bool,
) -> Result<Vec<SocketAddr>> {
    // 明示ポートが指定されたら SRV は見ない
    if let Some(p) = port {
        return lookup_host_with_port(resolver, host, p, force_v4).await;
    }

    // SRV: _minecraft._tcp.<host>
    let srv_name = format!("_minecraft._tcp.{host}.");
    if let Ok(srv) = resolver.srv_lookup(srv_name.clone()).await {
        let records: Vec<SRV> = srv.iter().cloned().collect();
        if !records.is_empty() {
            // RFC 2782: まず最小 priority のグループだけを対象にする
            let min_prio = records.iter().map(|r| r.priority()).min().unwrap();
            let lowest: Vec<SRV> = records
                .into_iter()
                .filter(|r| r.priority() == min_prio)
                .collect();

            // 各 target に対して A/AAAA を引き、その SRV の port を当てて候補にする
            let mut out = Vec::new();
            for r in lowest {
                let mut target = r.target().to_string(); // 末尾に '.' が付くことがある
                if target.ends_with('.') {
                    target.pop(); // 末尾 '.' を除去
                }
                let ips = resolver.lookup_ip(&target).await?;
                for ip in ips.iter() {
                    if force_v4 && ip.is_ipv6() {
                        continue;
                    }
                    out.push(SocketAddr::new(ip, r.port()));
                }
            }
            if !out.is_empty() {
                return Ok(out);
            }
            // SRV はあったが IP が引けない → フォールバック
        }
    }

    // SRV が無い／使えない → ホストの A/AAAA に 25565 を付与
    lookup_host_with_port(resolver, host, 25565, force_v4).await
}

async fn lookup_host_with_port(
    resolver: &TokioAsyncResolver,
    host: &str,
    port: u16,
    force_v4: bool,
) -> Result<Vec<SocketAddr>> {
    let ips = resolver.lookup_ip(host).await?;
    let mut v = Vec::new();
    for ip in ips.iter() {
        if force_v4 && ip.is_ipv6() {
            continue;
        }
        v.push(SocketAddr::new(ip, port));
    }
    Ok(v)
}

/* ---------- 同時レース接続（最初の成功を採用） ---------- */

async fn connect_race_all(
    addrs: &[SocketAddr],
    per_attempt: Duration,
) -> Result<(TcpStream, SocketAddr)> {
    let mut set = JoinSet::new();
    for addr in addrs.iter().copied() {
        set.spawn(async move {
            match timeout(per_attempt, TcpStream::connect(addr)).await {
                Ok(Ok(s)) => Ok::<_, io::Error>((s, addr)),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "connect timeout")),
            }
        });
    }

    let mut last_err: Option<io::Error> = None;

    while let Some(join_result) = set.join_next().await {
        match join_result {
            Ok(Ok((stream, addr))) => {
                set.abort_all();
                return Ok((stream, addr));
            }
            Ok(Err(e)) => last_err = Some(e),
            Err(join_err) => last_err = Some(io::Error::new(io::ErrorKind::Other, join_err)),
        }
    }

    Err(anyhow::anyhow!(last_err.unwrap_or_else(|| io::Error::new(
        io::ErrorKind::Other,
        "no addresses to try"
    ))))
    .context("All addresses failed concurrently")
}

fn join_addrs(addrs: &[SocketAddr]) -> String {
    addrs
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/* ---------- Protocol helpers & JSON ---------- */

fn write_varint(mut n: i32, out: &mut Vec<u8>) {
    loop {
        let mut temp = (n as u32 & 0b0111_1111) as u8;
        n = ((n as u32) >> 7) as i32;
        if n != 0 {
            temp |= 0b1000_0000;
        }
        out.push(temp);
        if n == 0 {
            break;
        }
    }
}

async fn read_varint<R: AsyncRead + Unpin>(r: &mut R) -> io::Result<i32> {
    let mut num_read = 0;
    let mut result: i32 = 0;
    loop {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf).await?;
        let byte = buf[0];
        let value = (byte & 0b0111_1111) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "VarInt too big"));
        }
        if (byte & 0b1000_0000) == 0 {
            break;
        }
    }
    Ok(result)
}

fn write_mc_string(s: &str, out: &mut Vec<u8>) {
    let bytes = s.as_bytes();
    write_varint(bytes.len() as i32, out);
    out.extend_from_slice(bytes);
}

async fn read_exact_n<R: AsyncRead + Unpin>(r: &mut R, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

impl fmt::Display for MinecraftServerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 必要なら MOTD の改行を潰すなどの整形をここで行う
        let motd = &self.motd;

        writeln!(f, "=== Minecraft Java Server Status ===")?;
        writeln!(
            f,
            "Address : {}:{} (resolved: {})",
            self.host, self.port_effective, self.resolved
        )?;
        writeln!(f, "Online  : YES (status retrieved)")?;
        writeln!(f, "Connect : ~{} ms", self.connect_ms)?;
        writeln!(f, "RTT     : ~{} ms (ping)", self.rtt_ms)?;
        writeln!(
            f,
            "Version : {} (protocol {})",
            self.version_name, self.version_protocol
        )?;
        writeln!(f, "Players : {}/{}", self.players_online, self.players_max)?;
        writeln!(f, "MOTD    : {}", motd)?;
        Ok(())
    }
}

/* ---------- JSON models ---------- */

#[derive(Debug, Deserialize)]
struct VersionInfo {
    name: String,
    protocol: i32,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct PlayersInfo {
    max: i32,
    online: i32,
    #[serde(default)]
    sample: Option<Vec<PlayerSample>>,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct PlayerSample {
    name: String,
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Description {
    Text(String),
    Obj(serde_json::Value),
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    version: VersionInfo,
    players: PlayersInfo,
    description: Description,
}

fn description_to_text(desc: &Description) -> String {
    match desc {
        Description::Text(s) => s.clone(),
        Description::Obj(v) => extract_text(v),
    }
}

fn extract_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => {
            let mut out = String::new();
            if let Some(t) = map.get("text").and_then(|x| x.as_str()) {
                out.push_str(t);
            }
            if let Some(arr) = map.get("extra").and_then(|x| x.as_array()) {
                for item in arr {
                    out.push_str(&extract_text(item));
                }
            }
            out
        }
        serde_json::Value::Array(arr) => arr.iter().map(extract_text).collect(),
        _ => String::new(),
    }
}
