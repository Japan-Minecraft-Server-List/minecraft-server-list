use std::sync::Arc;

use api::{
    serve,
    types::{API, Ordering, Server},
};
use async_trait::async_trait;
use tracing::info;

use crate::service::Service;

pub struct ApiServer {
    service: Arc<Service>,
}

impl ApiServer {
    pub fn new(service: Arc<Service>) -> Self {
        Self { service }
    }

    /// サーバーを起動する
    pub async fn serve(self) {
        serve(self, "localhost:3000").await.unwrap()
    }
}

#[async_trait]
impl API for ApiServer {
    /// サーバーリストを取得する
    /// 配列の順序はorderingに準拠する
    /// 定期的に更新するならキャッシュしても問題ない
    async fn get_server_list(&self, ordering: Ordering) -> Vec<Server> {
        info!("Recieved get_server_list ? ordering = {:?}", ordering);
        match ordering {
            Ordering::Player => {
                let players_order = self.service.online_players_order.read().unwrap().clone();
                players_order
                    .iter()
                    .map(|status| Server {
                        ip: status.ip.clone(),
                        name: status.name.clone(),
                        port: status.port as _,
                        description: status.description.clone(),
                        players_online: status.players_online as _,
                        players_max: status.players_max as _,
                    })
                    .collect()
            }
            Ordering::PlayerReverse => {
                let players_reverse_order = self
                    .service
                    .online_players_reverse_order
                    .read()
                    .unwrap()
                    .clone();
                players_reverse_order
                    .iter()
                    .map(|status| Server {
                        ip: status.ip.clone(),
                        name: status.name.clone(),
                        port: status.port as _,
                        description: status.description.clone(),
                        players_online: status.players_online as _,
                        players_max: status.players_max as _,
                    })
                    .collect()
            }
        }
    }
}
