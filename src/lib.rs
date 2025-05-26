#![allow(clippy::single_component_path_imports)]

mod aws;
pub use aws::SqsClientConfig;

mod runtime;
mod traits;

pub use runtime::Runtime as SqsConsumer;
