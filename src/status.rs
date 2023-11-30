use crate::{mutators, ShutdownReceiver, Tasks};
use axum::{
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use badge_maker::BadgeBuilder;
use spaceapi::Status;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

struct Inner {
    base_status: Status,
    rendered_status: Status,

    mutators: Vec<Box<dyn mutators::Mutator>>,
    mutator_data_notification_tx: mutators::DataNotificationSender,
}

#[derive(Clone)]
pub(crate) struct SpaceStatus {
    inner: Arc<Mutex<Inner>>,
    mqtt_client: mqtt_channel_client::Client,
}

impl SpaceStatus {
    pub(crate) async fn new(
        tasks: &mut Tasks,
        shutdown: ShutdownReceiver,
        base_status: Status,
        mqtt_client: mqtt_channel_client::Client,
    ) -> Self {
        let (mutator_data_notification_tx, mut mutator_data_notification_rx) =
            tokio::sync::mpsc::channel(32);

        // Send initial MQTT states
        crate::mqtt::send_status_state(&mqtt_client, &base_status).await;
        crate::mqtt::send_status(&mqtt_client, &base_status).await;

        let inner = Arc::new(Mutex::new(Inner {
            base_status: base_status.clone(),
            rendered_status: base_status,
            mutators: Vec::default(),
            mutator_data_notification_tx,
        }));

        let status = Self { inner, mqtt_client };

        tasks.spawn({
            let status = status.clone();

            async move {
                let mut new_data = false;

                loop {
                    if !shutdown.is_empty() {
                        break;
                    }

                    match tokio::time::timeout(mutators::UPDATE_TIMEOUT, async {
                        mutator_data_notification_rx.recv().await
                    })
                    .await
                    {
                        Ok(Some(())) => {
                            info!("New data from mutator");
                            crate::metrics::MUTATOR_DATA_UPDATES.inc();
                            new_data = true;
                        }
                        _ => {
                            if new_data {
                                status.update_from_mutators().await;
                                new_data = false;
                            }
                        }
                    }
                }
            }
        });

        status
    }

    #[tracing::instrument(skip(self))]
    async fn update_from_mutators(&self) {
        info!("Rendering status");
        crate::metrics::STATUS_RENDER_COUNT.inc();

        let mut inner = self.inner.lock().await;

        let mut new_status = inner.base_status.clone();

        for m in &inner.mutators {
            m.apply(&mut new_status);
        }

        if new_status.state != inner.rendered_status.state {
            info!("New status.state found after applying mutators");
            crate::mqtt::send_status_state(&self.mqtt_client, &new_status).await;
        }

        if new_status != inner.rendered_status {
            info!("New status found after applying mutators");
            crate::mqtt::send_status(&self.mqtt_client, &new_status).await;

            inner.rendered_status = new_status;
        }
    }

    pub(crate) async fn mutator_data_notification(&self) -> mutators::DataNotificationSender {
        self.inner.lock().await.mutator_data_notification_tx.clone()
    }

    pub(crate) async fn add_mutator(&self, mutator: Box<dyn mutators::Mutator>) {
        self.inner.lock().await.mutators.push(mutator);

        crate::metrics::MUTATORS.inc();
        info!("New mutator added");
    }

    async fn get(&self) -> Status {
        let state = self.inner.lock().await;
        state.rendered_status.clone()
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn http_get(&self) -> Json<Status> {
        crate::metrics::REQUESTS
            .get_or_create(&crate::metrics::RequestLabels::new(
                crate::metrics::Endpoint::SpaceApi,
            ))
            .inc();

        Json(self.get().await)
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn http_get_shield_simple(&self) -> Response {
        crate::metrics::REQUESTS
            .get_or_create(&crate::metrics::RequestLabels::new(
                crate::metrics::Endpoint::OpenShieldSimple,
            ))
            .inc();

        self.shield_status_common(BadgeBuilder::default()).await
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn http_get_shield_full(&self) -> Response {
        crate::metrics::REQUESTS
            .get_or_create(&crate::metrics::RequestLabels::new(
                crate::metrics::Endpoint::OpenShieldFull,
            ))
            .inc();

        let mut badge_builder = BadgeBuilder::default();

        let space = self.get().await.space;
        badge_builder.label(&space);

        self.shield_status_common(badge_builder).await
    }

    async fn shield_status_common(&self, mut badge_builder: BadgeBuilder) -> Response {
        let state = self.get().await.state.unwrap();
        let open = state.open.unwrap();

        if open {
            badge_builder
                .message(&match state.message {
                    Some(msg) => format!("Open ({})", msg),
                    None => "Open".to_string(),
                })
                .color(badge_maker::color::NamedColor::Green);
        } else {
            badge_builder
                .message("Closed")
                .color(badge_maker::color::NamedColor::Red);
        };

        let badge = badge_builder.build().unwrap().svg();

        ([(header::CONTENT_TYPE, "image/svg+xml")], badge).into_response()
    }
}
