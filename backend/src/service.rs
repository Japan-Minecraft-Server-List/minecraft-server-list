use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use futures::future::join_all;
use tokio::{fs::File, io::AsyncReadExt, time::sleep};
use tracing::{info, warn};

use crate::{config::ServersConfig, minecraft::MinecraftServerInfo};

/// ## Minecraftステータスチェッカーサービス
///
/// * 中身をRwLock<Arc<Vec<MinecraftServerStatus>>>で持っているのはクローンのコストを削減するため
/// * クローンコストを浮かせることでRwLockの保持が最短になり、並列度が高くなる
pub struct Service {
    pub online_players_order: RwLock<Arc<Vec<MinecraftServerStatus>>>,
    pub online_players_reverse_order: RwLock<Arc<Vec<MinecraftServerStatus>>>,
}

impl Service {
    pub fn new() -> Self {
        Self {
            online_players_order: RwLock::new(Arc::new(Vec::new())),
            online_players_reverse_order: RwLock::new(Arc::new(Vec::new())),
        }
    }

    pub async fn start(&self) {
        loop {
            info!("Getting server status...");

            // ファイルのopenを試みる
            let Ok(mut file) = File::open("./servers.toml").await else {
                warn!("Faileed to open servers.toml");
                warn!("Retry in 10 seconds...");

                sleep(Duration::from_secs(10)).await;
                continue;
            };

            // UTF-8読みを試みる
            let mut source = String::new();
            if let Err(_) = file.read_to_string(&mut source).await {
                warn!("Faileed to read as utf-8 servers.toml");
                warn!("Retry in 10 seconds...");

                sleep(Duration::from_secs(10)).await;
                continue;
            };

            // サーバーリストのTOMLファイルとしてパースする
            let servers_config = match ServersConfig::from_str(source.as_str()) {
                Ok(config) => config,
                Err(error) => {
                    warn!("Faileed to read servers.toml : {}", error);
                    warn!("Retry in 10 seconds...");

                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            // pingを飛ばす全タスク
            let tasks = servers_config.servers.iter().map(|server| async {
                MinecraftServerInfo::query(server.ip.as_str(), server.port).await
            });

            // 一斉にpingを飛ばしてすべての結果を待つ
            let server_status = join_all(tasks).await;

            let mut final_server_status = Vec::new();

            // (pingを飛ばすのに失敗した場合はoffline判定)
            for (server, status) in servers_config.servers.iter().zip(server_status.into_iter()) {
                match status {
                    Ok(status) => {
                        final_server_status.push(MinecraftServerStatus {
                            ip: server.ip.clone(),
                            port: status.port_effective as _,
                            icon: server.icon.clone(),
                            name: server.name.clone(),
                            description: server.description.clone(),
                            is_online: true,
                            version_name: status.version_name,
                            players_online: status.players_online,
                            players_max: status.players_max,
                        });
                    }
                    Err(_) => {
                        final_server_status.push(MinecraftServerStatus {
                            ip: server.ip.clone(),
                            port: 25565,
                            icon: server.icon.clone(),
                            name: server.name.clone(),
                            description: server.description.clone(),
                            is_online: false,
                            version_name: "".to_string(),
                            players_online: 0,
                            players_max: 0,
                        });
                    }
                }
            }

            // 人数の少ない順にソート
            let mut players_reverse_order = final_server_status.clone();
            players_reverse_order.sort_by(|a, b| a.players_online.cmp(&b.players_online));

            // ソート結果を逆にして人数の多い順を作る
            let mut players_order = players_reverse_order.clone();
            players_order.reverse();

            // 結果を反映する
            *self.online_players_order.write().unwrap() = Arc::new(players_order);
            *self.online_players_reverse_order.write().unwrap() = Arc::new(players_reverse_order);

            info!("Getting status is completed!");

            // 指定の秒数待って繰り返す
            sleep(Duration::from_secs(10)).await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinecraftServerStatus {
    pub ip: String,
    pub port: i32,
    pub icon: String,
    pub name: String,
    pub description: String,
    pub is_online: bool,
    pub version_name: String,
    pub players_online: i32,
    pub players_max: i32,
}
