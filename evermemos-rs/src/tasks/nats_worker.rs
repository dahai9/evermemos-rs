use anyhow::{Context, Result};
use async_nats::jetstream::{self, consumer::PullConsumer};
use futures::StreamExt;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::biz::memorize::{MemorizeRequest, MemorizeService, RawMessage};
use crate::config::NatsConfig;
use super::task_types::{MemorizeTask, TaskPayload};

/// NATS JetStream worker.
/// Subscribes to the configured subject and dispatches tasks to `MemorizeService`.
pub struct NatsWorker {
    memorize_svc: Arc<MemorizeService>,
    cfg: NatsConfig,
}

impl NatsWorker {
    pub fn new(memorize_svc: Arc<MemorizeService>, cfg: NatsConfig) -> Self {
        Self { memorize_svc, cfg }
    }

    /// Connect to NATS, set up JetStream stream/consumer, and start polling.
    /// This **blocks** until the worker shuts down.
    pub async fn start(self) -> Result<()> {
        let client = async_nats::connect(&self.cfg.url)
            .await
            .context("Failed to connect to NATS")?;

        info!("NATS worker connected to {}", self.cfg.url);

        let js = jetstream::new(client);

        // Create (or get existing) stream
        let stream = js
            .get_or_create_stream(jetstream::stream::Config {
                name: self.cfg.stream.clone(),
                subjects: vec![self.cfg.subject_memorize.clone()],
                ..Default::default()
            })
            .await
            .context("Failed to ensure NATS stream")?;

        // Durable pull consumer
        let consumer: PullConsumer = stream
            .get_or_create_consumer(
                "evermemos-worker",
                jetstream::consumer::pull::Config {
                    durable_name: Some("evermemos-worker".into()),
                    ..Default::default()
                },
            )
            .await
            .context("Failed to create NATS consumer")?;

        info!(
            "NATS worker consuming from stream={} subject={}",
            self.cfg.stream, self.cfg.subject_memorize
        );

        loop {
            let mut batch = match consumer.fetch().max_messages(10).messages().await {
                Ok(b) => b,
                Err(e) => {
                    error!("NATS fetch error: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            while let Some(msg) = batch.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("NATS message error: {e}");
                        continue;
                    }
                };

                // Parse payload
                let payload: TaskPayload = match serde_json::from_slice(&msg.payload) {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("NATS: failed to parse message payload: {e}");
                        let _ = msg.ack().await;
                        continue;
                    }
                };

                match payload {
                    TaskPayload::Memorize(task) => {
                        if let Err(e) = self.process_memorize(task).await {
                            error!("Memorize task failed: {e}");
                        }
                    }
                    TaskPayload::Sync(task) => {
                        info!("Sync task received for user={} reason={}", task.user_id, task.reason);
                        // Sync logic can be added here later
                    }
                }

                let _ = msg.ack().await;
            }

            // Brief yield so the loop doesn't busy-spin on empty batches
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    async fn process_memorize(&self, task: MemorizeTask) -> Result<()> {
        let req = MemorizeRequest {
            message: RawMessage {
                message_id: task.message_id,
                sender: task.sender,
                sender_name: task.sender_name,
                content: task.content,
                create_time: task.create_time,
                role: task.role,
            },
            user_id: task.user_id,
            user_name: task.user_name,
            group_id: task.group_id,
            group_name: task.group_name,
            history: task.history,
        };

        let result = self.memorize_svc.memorize(req).await?;
        info!("NATS: memorize done status={} count={}", result.status, result.saved_count);
        Ok(())
    }
}
