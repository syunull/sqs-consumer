use async_trait::async_trait;
use aws_sdk_sqs::{
    error::SdkError,
    operation::{
        delete_message_batch::{DeleteMessageBatchError, DeleteMessageBatchOutput},
        receive_message::{ReceiveMessageError, ReceiveMessageOutput},
    },
    types::DeleteMessageBatchRequestEntry,
};

#[async_trait]
pub(crate) trait SqsClient {
    async fn receive_message(&self) -> Result<ReceiveMessageOutput, SdkError<ReceiveMessageError>>;
    async fn delete_message_batch(
        &self,
        receipts: &mut Vec<DeleteMessageBatchRequestEntry>,
    ) -> Result<DeleteMessageBatchOutput, SdkError<DeleteMessageBatchError>>;
}
