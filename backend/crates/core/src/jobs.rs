//! Việc nền: `api` đẩy việc vào RabbitMQ, `worker` lấy ra xử lý.
//!
//! Trong test dùng hàng đợi bộ nhớ (`JobQueue::memory()`) để đọc lại việc đã đẩy,
//! ví dụ đọc mã OTP trong email.

use std::sync::{Arc, Mutex};

use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties,
    options::{BasicPublishOptions, ConfirmSelectOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable},
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Tên hàng đợi email.
pub const QUEUE_EMAIL: &str = "sls.email";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Job {
    SendEmail(Email),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
}

#[derive(Clone)]
pub enum JobQueue {
    Amqp { channel: Channel },
    Memory(Arc<Mutex<Vec<Job>>>),
}

impl JobQueue {
    /// Kết nối RabbitMQ, khai báo hàng đợi, bật publisher confirm.
    pub async fn connect_amqp(url: &str) -> Result<(Self, Connection), lapin::Error> {
        let conn = Connection::connect(url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;
        declare_queues(&channel).await?;
        channel.confirm_select(ConfirmSelectOptions::default()).await?;
        Ok((Self::Amqp { channel }, conn))
    }

    pub fn memory() -> Self {
        Self::Memory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn publish(&self, job: Job) -> AppResult<()> {
        match self {
            Self::Amqp { channel } => {
                let payload = serde_json::to_vec(&job).map_err(AppError::internal)?;
                let queue = match job {
                    Job::SendEmail(_) => QUEUE_EMAIL,
                };
                channel
                    .basic_publish(
                        "".into(),
                        queue.into(),
                        BasicPublishOptions::default(),
                        &payload,
                        // delivery_mode 2 = lưu xuống đĩa, không mất khi RabbitMQ khởi động lại
                        BasicProperties::default().with_delivery_mode(2),
                    )
                    .await
                    .map_err(AppError::internal)?
                    .await
                    .map_err(AppError::internal)?;
                Ok(())
            }
            Self::Memory(jobs) => {
                jobs.lock().expect("mutex").push(job);
                Ok(())
            }
        }
    }

    /// Lấy ra (và xóa) các việc đã đẩy vào hàng đợi bộ nhớ. Chỉ dùng trong test.
    pub fn drain_memory(&self) -> Vec<Job> {
        match self {
            Self::Memory(jobs) => std::mem::take(&mut *jobs.lock().expect("mutex")),
            Self::Amqp { .. } => Vec::new(),
        }
    }

    /// Kiểm tra kết nối RabbitMQ còn sống (dùng cho health check).
    pub fn is_healthy(&self) -> bool {
        match self {
            Self::Amqp { channel } => channel.status().connected(),
            Self::Memory(_) => true,
        }
    }
}

/// Khai báo các hàng đợi (quorum queue, bền vững). Gọi được nhiều lần.
pub async fn declare_queues(channel: &Channel) -> Result<(), lapin::Error> {
    let mut args = FieldTable::default();
    args.insert("x-queue-type".into(), AMQPValue::LongString("quorum".into()));
    channel.queue_declare(QUEUE_EMAIL.into(), QueueDeclareOptions::durable(), args).await?;
    Ok(())
}
