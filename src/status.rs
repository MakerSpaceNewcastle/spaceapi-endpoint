use crate::{mutators, utils::shieldsio_static_sanitise, ShutdownReceiver, Tasks};
use axum::{response::Redirect, Json};
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
    pub(crate) async fn http_get_shield_simple(&self) -> Redirect {
        crate::metrics::REQUESTS
            .get_or_create(&crate::metrics::RequestLabels::new(
                crate::metrics::Endpoint::OpenShieldSimple,
            ))
            .inc();

        let status = self.get().await;
        let state = status.state.unwrap();

        let msg = shieldsio_static_sanitise(
            if state.open.unwrap() {
                "Open"
            } else {
                "Closed"
            }
            .to_string(),
        );

        let colour = if state.open.unwrap() { "green" } else { "red" };

        Redirect::to(&format!("https://img.shields.io/badge/-{msg}-{colour}"))
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn http_get_shield_full(&self) -> Redirect {
        crate::metrics::REQUESTS
            .get_or_create(&crate::metrics::RequestLabels::new(
                crate::metrics::Endpoint::OpenShieldFull,
            ))
            .inc();

        let status = self.get().await;
        let state = status.state.unwrap();

        let space_name = shieldsio_static_sanitise(status.space);

        let msg = shieldsio_static_sanitise(if state.open.unwrap() {
            match state.message {
                Some(msg) => format!("Open ({})", msg),
                None => "Open".to_string(),
            }
        } else {
            "Closed".to_string()
        });

        let colour = if state.open.unwrap() { "green" } else { "red" };

        Redirect::to(&format!(
            "https://img.shields.io/badge/{space_name}-{msg}-{colour}"
        ))
    }
}
