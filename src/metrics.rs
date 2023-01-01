use lazy_static::lazy_static;
use prometheus_client::{
    encoding::{EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family},
};

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
}

lazy_static! {
    pub(crate) static ref REQUESTS: Family::<RequestLabels, Counter> =
        Family::<RequestLabels, Counter>::default();
}
