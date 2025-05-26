use std::sync::Arc;

use aws_sdk_sqs::types::Message;
use tokio::sync::mpsc::Sender;
use tokio::sync::Notify;

use crate::traits::SqsClient;

pub(crate) struct Poller<T> {
    channel: Sender<Message>,
    notify: Arc<Notify>,
    client: T,
}

impl<T> Poller<T> {
    pub fn new(notify: Arc<Notify>, channel: Sender<Message>, client: T) -> Self {
        Self {
            notify,
            channel,
            client,
        }
    }
}

impl<T> Poller<T>
where
    T: SqsClient,
{
    pub async fn poll_aws_sqs_messages(self) {
        let notify = self.notify.clone();
        tokio::pin!(notify);

        loop {
            tokio::select! {
                _ = notify.notified() => {
                    tracing::info!("stopping polling worker");
                    break
                }

                Ok(messages) = self.client.receive_message() => {
                    if let Some(messages) = messages.messages {
                        for message in messages {
                            if let Err(e) = self.channel.send(message).await {
                                tracing::error!("failed to send internal message: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}
