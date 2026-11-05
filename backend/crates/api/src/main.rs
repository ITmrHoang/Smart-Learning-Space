//! Binary `api`: đọc cấu hình, kết nối hạ tầng, chạy migration, tạo admin đầu tiên, mở cổng HTTP.

use std::net::SocketAddr;

use anyhow::Context;
use sls_api::app::build_router;
use sls_core::{
    AppContext,
    config::Config,
    db::{self, MIGRATOR},
    jobs::JobQueue,
    modules::users::service as users,
    rate_limit::RateLimiter,
};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    sls_core::config::load_dotenv();
    init_tracing();

    let config = Config::from_env()?;
    tracing::info!(?config, "khởi động api");

    let pool = db::connect(&config.database_url).await.context("không kết nối được PostgreSQL")?;
    MIGRATOR.run(&pool).await.context("chạy migration thất bại")?;

    let valkey = redis::Client::open(config.valkey_url.as_str())?
        .get_connection_manager()
        .await
        .context("không kết nối được Valkey")?;
    let (jobs, _amqp_conn) = JobQueue::connect_amqp(&config.amqp_url).await.context("không kết nối được RabbitMQ")?;

    let bind_addr = config.bind_addr.clone();
    let ctx = AppContext::new(config, pool, jobs, RateLimiter::Valkey(valkey.clone()), Some(valkey));
    users::bootstrap_admin(&ctx).await.map_err(|e| anyhow::anyhow!("tạo admin đầu tiên thất bại: {e}"))?;

    let app = build_router(ctx);
    let listener =
        tokio::net::TcpListener::bind(&bind_addr).await.with_context(|| format!("không mở được {bind_addr}"))?;
    tracing::info!("API đang chạy tại http://{bind_addr}");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("đang dừng api…");
        })
        .await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn"));
    if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        fmt().with_env_filter(filter).json().init();
    } else {
        fmt().with_env_filter(filter).init();
    }
}
