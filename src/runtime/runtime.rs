use std::future::Future;
use std::sync::Arc;

use aws_sdk_sqs::types::{DeleteMessageBatchRequestEntry, Message};
use tokio::sync::mpsc::Sender;
use tokio::sync::{Notify, Semaphore};
use tokio_util::task::TaskTracker;

use crate::aws::{AwsSqsClient, SqsClientConfig};
use crate::runtime::{BatchDeleter, Poller, SignalManager, Timer};

static DEFAULT_AWS_SQS_CONSUMER_POLLER_COUNT: usize = 1;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

pub struct Runtime<T> {
    set: TaskTracker,
    poller_count: usize,
    future_fn: T,

    sqs_client_config: SqsClientConfig,
}

impl<T> Runtime<T> {
    pub fn new(future: T, sqs_client_config: SqsClientConfig) -> Self {
        let poller_count = match std::env::var("AWS_SQS_CONSUMER_POLLER_COUNT") {
            Ok(env) => env.parse().unwrap_or(DEFAULT_AWS_SQS_CONSUMER_POLLER_COUNT),
            Err(_) => DEFAULT_AWS_SQS_CONSUMER_POLLER_COUNT,
        };

        Self {
            poller_count,
            sqs_client_config,
            set: TaskTracker::new(),
            future_fn: future,
        }
    }
}

impl<F, Fut> Runtime<F>
where
    F: Fn(Message) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    #[tracing::instrument(name = "runtime", skip(self))]
    pub async fn start(self) -> Result<()> {
        let sqs_client = AwsSqsClient::from(self.sqs_client_config);

        let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel::<Message>(100);
        let (del_tx, del_rx) = tokio::sync::mpsc::channel::<DeleteMessageBatchRequestEntry>(100);

        let notify = Arc::new(Notify::new());

        (0..self.poller_count).for_each(|i| {
            tracing::info!("starting poller {}", i + 1);
            self.set
                .spawn(Poller::new(notify.clone(), msg_tx.clone(), sqs_client.clone()).poll_aws_sqs_messages());
        });

        self.set
            .spawn(BatchDeleter::new(notify.clone(), del_rx, sqs_client.clone()).aws_sqs_delete_message_batch());

        self.set.spawn(SignalManager::new(notify.clone()));

        let max_concurrency = self.poller_count * 4 + 1;
        let semaphore = Arc::new(Semaphore::new(max_concurrency));

        loop {
            tokio::select! {
                _ = notify.notified() => {
                    tracing::info!("received shutdown signal.");
                    break;
                }

                Some(msg) = msg_rx.recv() => {
                    let semaphore = semaphore.clone();
                    let permit = match semaphore.acquire_owned().await {
                        Ok(permit) => permit,
                        Err(e) => {
                            tracing::error!("internal error getting permit: {}", e);
                            continue;
                        }
                    };

                    let future_fn = self.future_fn.clone();
                    let del_tx = del_tx.clone();
                    self.set.spawn(async move {
                        Self::run_internal(future_fn, msg, del_tx).await;
                        drop(permit);
                    }
                    );
                }
            }
        }
        // consider draining msg_rx channel
        drop(del_tx);

        self.set.close();
        self.set.wait().await;
        Ok(())
    }

    #[tracing::instrument(name = "runtime", skip(future, message, channel), fields(id = %message.message_id.as_deref().unwrap_or("unknown")))]
    async fn run_internal(future: F, message: Message, channel: Sender<DeleteMessageBatchRequestEntry>) {
        let Some(id) = &message.message_id else {
            tracing::error!("message missing message id {:?}", &message);
            return;
        };
        let Some(_body) = &message.body else {
            tracing::error!("message missing body {:?}", &message);
            return;
        };
        let Some(receipt) = &message.receipt_handle else {
            tracing::error!("message missing receipt handle {:?}", &message);
            return;
        };

        let id = id.clone();
        let receipt = receipt.clone();

        match Timer::new(future(message)).await {
            Ok(_) => {
                let batch_entry = match DeleteMessageBatchRequestEntry::builder()
                    .id(id.as_str())
                    .receipt_handle(receipt)
                    .build()
                {
                    Ok(batch_entry) => batch_entry,
                    Err(e) => {
                        tracing::error!("error building batch entry: {}", e);
                        return;
                    }
                };

                if let Err(e) = channel.send(batch_entry).await {
                    tracing::error!("error sending batch entry: {}", e);
                };
            }
            Err(e) => {
                tracing::error!("error processing message: {}", e);
            }
        }
    }
}
