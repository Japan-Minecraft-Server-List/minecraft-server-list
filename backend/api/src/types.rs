/// サーバーリストの順序
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Ordering {
    /// プレイヤーの多い順

Player,
    /// プレイヤーの少ない順

PlayerReverse,
}

/// サーバーリストの要素
#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Server {
    /// Minecraftサーバーのポート
    pub port: i64,
    /// MinecraftサーバーのIPアドレス
    pub ip: String,
    /// サーバーの名前
    pub name: String,
    /// プレイヤー人数
    pub players_online: i64,
    /// サーバーの説明欄
    /// 改行可
    pub description: String,
    /// 最大プレイ人数
    pub players_max: i64,
}



use serde::{Serialize, Deserialize};
use async_trait::async_trait;

#[async_trait]
pub trait API: Send + Sync + 'static {
    /// サーバーリストを取得する
    /// 配列の順序はorderingに準拠する
    /// 定期的に更新するならキャッシュしても問題ない
    async fn get_server_list(&self, ordering: Ordering) -> Vec<Server>;
}

