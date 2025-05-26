use std::sync::OnceLock;

use opentelemetry::metrics::{Counter, Histogram};

static TASK_DURATION_HISTOGRAM: OnceLock<Histogram<u64>> = OnceLock::new();
pub(crate) fn get_task_duration_histogram() -> &'static Histogram<u64> {
    TASK_DURATION_HISTOGRAM.get_or_init(|| {
        let meter = opentelemetry::global::meter("task_duration_seconds_histogram");
        meter
            .u64_histogram("task_duration_seconds")
            .with_description("Duration of asynchronous tasks in seconds")
            .with_unit("s")
            .build()
    })
}

static TASK_DELETE_SUCCESS_COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
pub(crate) fn get_task_delete_success_counter() -> &'static Counter<u64> {
    TASK_DELETE_SUCCESS_COUNTER.get_or_init(|| {
        let meter = opentelemetry::global::meter("task_delete_success_counter");
        meter
            .u64_counter("task_delete_success_total")
            .with_description("Total number of tasks deleted successfully")
            .build()
    })
}

static TASK_DELETE_FAILURE_COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
pub(crate) fn get_task_delete_failure_counter() -> &'static Counter<u64> {
    TASK_DELETE_FAILURE_COUNTER.get_or_init(|| {
        let meter = opentelemetry::global::meter("task_delete_failure_counter");
        meter
            .u64_counter("task_delete_failure_total")
            .with_description("Total number of tasks failed to delete")
            .build()
    })
}
