use std::sync::Arc;
use std::time::Duration;

use aws_sdk_sqs::types::DeleteMessageBatchRequestEntry;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Notify;
use tokio::time::interval;

use crate::runtime::metrics::{get_task_delete_failure_counter, get_task_delete_success_counter};
use crate::traits::SqsClient;

pub(crate) struct BatchDeleter<T> {
    channel: Receiver<DeleteMessageBatchRequestEntry>,
    notify: Arc<Notify>,
    buffer: Vec<DeleteMessageBatchRequestEntry>,

    client: T,
}

impl<T> BatchDeleter<T> {
    pub fn new(notify: Arc<Notify>, channel: Receiver<DeleteMessageBatchRequestEntry>, client: T) -> Self {
        Self {
            notify,
            channel,
            client,
            buffer: Vec::with_capacity(20),
        }
    }
}

impl<T> BatchDeleter<T>
where
    T: SqsClient,
{
    pub async fn aws_sqs_delete_message_batch(mut self) {
        let notify = self.notify.clone();
        tokio::pin!(notify);

        let mut ticker = interval(Duration::from_secs(2));

        loop {
            tokio::select! {
                _ = notify.notified() => {
                    break
                }
                _ = ticker.tick() => {
                    while !self.buffer.is_empty() {
                        self.delete_message_batch_internal().await;
                    }
                }
                Some(msg) = self.channel.recv() => {
                    self.buffer.push(msg);
                    if self.buffer.len() >= 10 {
                        self.delete_message_batch_internal().await;
                    }
                }
            }
        }

        while let Some(msg) = self.channel.recv().await {
            self.buffer.push(msg);
            if self.buffer.len() >= 10 {
                self.delete_message_batch_internal().await;
            }
        }
        while !self.buffer.is_empty() {
            self.delete_message_batch_internal().await;
        }
    }

    async fn delete_message_batch_internal(&mut self) {
        match self.client.delete_message_batch(&mut self.buffer).await {
            Ok(resp) => {
                for message in resp.failed() {
                    let span = tracing::span!(tracing::Level::ERROR, "runtime", id = %message.id);
                    let _guard = span.enter();
                    get_task_delete_failure_counter().add(1, &[]);
                    tracing::error!("delete message failure");
                }

                for message in resp.successful() {
                    let span = tracing::span!(tracing::Level::INFO, "runtime", id = %message.id);
                    let _guard = span.enter();
                    get_task_delete_success_counter().add(1, &[]);
                    tracing::info!("delete message successful");
                }
            }
            Err(e) => {
                tracing::error!("Failed to delete message batch: {:?}", e);
            }
        }
    }
}
