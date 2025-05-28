use std::sync::Arc;

use async_trait::async_trait;
use aws_config::SdkConfig;
use aws_sdk_sqs::{
    error::SdkError,
    operation::{
        delete_message_batch::{DeleteMessageBatchError, DeleteMessageBatchOutput},
        receive_message::{ReceiveMessageError, ReceiveMessageOutput},
    },
    types::DeleteMessageBatchRequestEntry,
    Client,
};

use crate::traits::SqsClient;

pub struct SqsClientConfig {
    config: SdkConfig,
    url: String,
    max_number_of_messages: i32,
    wait_time_seconds: i32,
}

impl SqsClientConfig {
    pub fn new(config: SdkConfig, url: impl Into<String>) -> Self {
        Self {
            config,
            url: url.into(),
            max_number_of_messages: 10,
            wait_time_seconds: 20,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AwsSqsClient {
    inner: Arc<Client>,
    queue_url: String,
    max_number_of_messages: i32,
    wait_time_seconds: i32,
}

impl From<SqsClientConfig> for AwsSqsClient {
    fn from(value: SqsClientConfig) -> Self {
        Self {
            inner: Arc::new(Client::new(&value.config)),
            queue_url: value.url,
            max_number_of_messages: value.max_number_of_messages,
            wait_time_seconds: value.wait_time_seconds,
        }
    }
}

#[async_trait]
impl SqsClient for AwsSqsClient {
    async fn receive_message(&self) -> Result<ReceiveMessageOutput, SdkError<ReceiveMessageError>> {
        self.inner
            .receive_message()
            .queue_url(&self.queue_url)
            .max_number_of_messages(self.max_number_of_messages)
            .wait_time_seconds(self.wait_time_seconds)
            .send()
            .await
    }

    async fn delete_message_batch(
        &self,
        receipts: &mut Vec<DeleteMessageBatchRequestEntry>,
    ) -> Result<DeleteMessageBatchOutput, SdkError<DeleteMessageBatchError>> {
        let batch_size = std::cmp::min(10usize, receipts.len());

        // messages in sqs are not guaranteed to be unique, so we need to remove duplicates
        let mut receipts: Vec<DeleteMessageBatchRequestEntry> = receipts.drain(..batch_size).collect();
        let before_count = receipts.len();
        receipts.sort_by(|a, b| a.id.cmp(&b.id));
        receipts.dedup_by(|a, b| a.id.eq(&b.id));
        let after_count = receipts.len();

        if before_count != after_count {
            tracing::warn!("Duplicated receipts removed");
        }

        self.inner
            .delete_message_batch()
            .queue_url(&self.queue_url)
            .set_entries(Some(receipts))
            .send()
            .await
    }
}
