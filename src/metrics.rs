use kagiyama::prometheus as prometheus_client;
use kagiyama::prometheus::{
    encoding::{EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family},
    registry::Registry,
};
use lazy_static::lazy_static;

#[derive(Debug, Clone, Eq, Hash, PartialEq, EncodeLabelSet)]
pub(crate) struct RequestLabels {
    endpoint: Endpoint,
}

impl RequestLabels {
    pub(crate) fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, EncodeLabelValue)]
pub(crate) enum Endpoint {
    SpaceApi,
    OpenBadgeSimple,
    OpenBadgeFull,
}

lazy_static! {
    pub(crate) static ref REQUESTS: Family::<RequestLabels, Counter> =
        Family::<RequestLabels, Counter>::default();
    pub(crate) static ref STATUS_RENDER_COUNT: Counter = Counter::default();
    pub(crate) static ref MUTATORS: Counter = Counter::default();
    pub(crate) static ref MUTATOR_DATA_UPDATES: Counter = Counter::default();
    pub(crate) static ref MUTATOR_ERRORS: Counter = Counter::default();
}

pub(crate) fn register_metrics(registry: &mut Registry) {
    registry.register("requests", "SpaceAPI requests", REQUESTS.clone());

    registry.register(
        "status_render",
        "Number of times the status has been rendered",
        STATUS_RENDER_COUNT.clone(),
    );

    registry.register(
        "mutators",
        "Number of registered mutators",
        MUTATORS.clone(),
    );

    registry.register(
        "mutator_data_updates",
        "Number of data updates handled by mutators",
        MUTATOR_DATA_UPDATES.clone(),
    );

    registry.register(
        "mutator_errors",
        "Number of errors encountered by mutators when processing new data",
        MUTATOR_ERRORS.clone(),
    );
}
