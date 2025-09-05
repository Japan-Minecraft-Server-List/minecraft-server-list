use std::sync::Arc;

use tokio::spawn;
use tracing::info;

use crate::{log::setup_tracing, server::ApiServer, service::Service};

pub mod config;
pub mod log;
pub mod minecraft;
pub mod server;
pub mod service;

#[tokio::main]
async fn main() {
    // ログのセットアップ
    let _log_guard = setup_tracing();

    // Minecraftサーバーのステータスチェッカー
    let service = Arc::new(Service::new());

    let service_ = service.clone();

    // ステータスチェッカーを起動する
    spawn(async move {
        service_.start().await;
    });

    // APIサーバー
    let server = ApiServer::new(service);

    info!("Starting server...");

    // APIサーバーを起動する
    server.serve().await;
}
