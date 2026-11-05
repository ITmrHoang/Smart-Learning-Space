//! Binary `worker`: lấy việc từ RabbitMQ và xử lý. Hiện có: gửi email.
//!
//! Gửi lỗi thì thử lại tối đa 3 lần (chờ 2s, 4s); vẫn lỗi thì bỏ qua và ghi log
//! (hàng đợi dead-letter sẽ thêm khi có nhiều loại việc hơn).

use std::time::Duration;

use anyhow::Context;
use futures_util::StreamExt;
use lapin::{
    Connection, ConnectionProperties,
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions},
    types::FieldTable,
};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use sls_core::jobs::{Email, Job, QUEUE_EMAIL, declare_queues};
use tracing_subscriber::{EnvFilter, fmt};

struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Mailer {
    fn from_env() -> anyhow::Result<Self> {
        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port: u16 = std::env::var("SMTP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(1025);
        let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "Smart Learning Space <no-reply@sls.local>".into());
        // Dev dùng Mailpit (SMTP không mã hóa). Production cần SMTP có TLS: sẽ bật feature TLS của lettre.
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port);
        if let (Ok(user), Ok(pass)) = (std::env::var("SMTP_USERNAME"), std::env::var("SMTP_PASSWORD")) {
            builder = builder.credentials(Credentials::new(user, pass));
        }
        Ok(Self { transport: builder.build(), from: from.parse().context("SMTP_FROM không hợp lệ")? })
    }

    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        let message = Message::builder()
            .from(self.from.clone())
            .to(email.to.parse().context("địa chỉ nhận không hợp lệ")?)
            .subject(&email.subject)
            .header(ContentType::TEXT_PLAIN)
            .body(email.text.clone())?;
        self.transport.send(message).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    sls_core::config::load_dotenv();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        fmt().with_env_filter(filter).json().init();
    } else {
        fmt().with_env_filter(filter).init();
    }

    let amqp_url = std::env::var("AMQP_URL").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());
    let mailer = Mailer::from_env()?;

    let conn =
        Connection::connect(&amqp_url, ConnectionProperties::default()).await.context("không kết nối được RabbitMQ")?;
    let channel = conn.create_channel().await?;
    declare_queues(&channel).await?;
    channel.basic_qos(5, BasicQosOptions::default()).await?;
    let mut consumer = channel
        .basic_consume(QUEUE_EMAIL.into(), "sls-worker".into(), BasicConsumeOptions::default(), FieldTable::default())
        .await?;
    tracing::info!(queue = QUEUE_EMAIL, "worker đang chờ việc");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("đang dừng worker…");
                break;
            }
            next = consumer.next() => {
                let Some(delivery) = next else { break };
                let delivery = delivery?;
                match serde_json::from_slice::<Job>(&delivery.data) {
                    Ok(Job::SendEmail(email)) => {
                        if send_with_retry(&mailer, &email).await {
                            delivery.ack(BasicAckOptions::default()).await?;
                        } else {
                            delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await?;
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "việc không đọc được, bỏ qua");
                        delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn send_with_retry(mailer: &Mailer, email: &Email) -> bool {
    for attempt in 1..=3u32 {
        match mailer.send(email).await {
            Ok(()) => {
                tracing::info!(to = %email.to, subject = %email.subject, "đã gửi email");
                return true;
            }
            Err(e) => {
                tracing::warn!(attempt, to = %email.to, error = %e, "gửi email thất bại");
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
        }
    }
    tracing::error!(to = %email.to, "bỏ email sau 3 lần thử");
    false
}
