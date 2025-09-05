use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, fmt::time::LocalTime, layer::SubscriberExt, util::SubscriberInitExt,
};

pub fn setup_tracing() -> WorkerGuard {
    // RUST_LOG があればそれを使い、なければデフォルト
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hyper=warn"));

    // 出力先（存在しなければ作る）
    let _ = std::fs::create_dir_all("logs");
    let app_file = tracing_appender::rolling::daily("logs", "app.log");
    let (app_nb, app_guard) = tracing_appender::non_blocking(app_file);

    // コンソール
    let console_layer = tracing_subscriber::fmt::layer()
        .with_timer(LocalTime::rfc_3339())
        .with_target(false)
        .pretty()
        .with_writer(std::io::stdout);

    // ファイル
    let app_json_layer = tracing_subscriber::fmt::layer()
        .with_timer(LocalTime::rfc_3339())
        .with_ansi(false)
        .with_writer(app_nb);

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(app_json_layer)
        .try_init()
        .expect("already exists");

    app_guard
}
