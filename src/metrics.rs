use kagiyama::prometheus as prometheus_client;
use kagiyama::prometheus::{
    encoding::{EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family},
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
    OpenShield,
}

lazy_static! {
    pub(crate) static ref REQUESTS: Family::<RequestLabels, Counter> =
        Family::<RequestLabels, Counter>::default();
}
