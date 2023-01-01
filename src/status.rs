use axum::Json;
use spaceapi::Status;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

struct InnerState {
    status: Status,
    mutators: Vec<Mutator>,
}

#[derive(Clone)]
pub(crate) struct SpaceStatus {
    state: Arc<Mutex<InnerState>>,
    _mqtt_update_task: Arc<Mutex<JoinHandle<()>>>,
    mqtt_client: mqtt_channel_client::Client,
}

impl SpaceStatus {
    pub(crate) fn new(status: Status, mqtt_client: mqtt_channel_client::Client) -> Self {
        let state = Arc::new(Mutex::new(InnerState {
            status,
            mutators: Vec::default(),
        }));

        let mqtt_update_task = {
            let client = mqtt_client.clone();
            let state = state.clone();
            Arc::new(Mutex::new(tokio::spawn(async move {
                let mut rx = client.rx_channel();
                loop {
                    while let Ok(mqtt_channel_client::Event::Rx(msg)) = rx.recv().await {
                        debug!("New MQTT message");
                        let mut state = state.lock().unwrap();
                        let mutators = state.mutators.clone();
                        for m in mutators {
                            m.handle_mqtt_message(&mut state.status, &msg);
                        }
                    }
                }
            })))
        };

        Self {
            state,
            _mqtt_update_task: mqtt_update_task,
            mqtt_client,
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn add_mutator(&self, mutator: Mutator) {
        self.mqtt_client.subscribe(
            mqtt_channel_client::SubscriptionBuilder::default()
                .topic(mutator.topic.clone())
                .build()
                .unwrap(),
        );
        self.state.lock().unwrap().mutators.push(mutator);
        info!("New mutator added");
    }

    #[tracing::instrument(skip(self))]
    fn get(&self) -> Status {
        let state = self.state.lock().unwrap();
        state.status.clone()
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn http_get(&self) -> Json<Status> {
        crate::metrics::REQUESTS
            .get_or_create(&crate::metrics::RequestLabels::new(
                crate::metrics::Endpoint::SpaceApi,
            ))
            .inc();
        Json(self.get())
    }
}

impl SpaceStatus {
    #[tracing::instrument(skip(self))]
    fn ensure_sensors_struct(&self) {
        let mut state = self.state.lock().unwrap();
        if state.status.sensors.is_none() {
            debug!("Adding empty sensors struct");
            state.status.sensors = Some(spaceapi::sensors::Sensors::default());
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn add_temperature_sensor(
        &self,
        name: &str,
        location: &str,
        description: &str,
        topic: &str,
    ) {
        self.ensure_sensors_struct();

        {
            let mut state = self.state.lock().unwrap();
            state.status.sensors.as_mut().unwrap().temperature.push(
                spaceapi::sensors::TemperatureSensor {
                    metadata: spaceapi::sensors::SensorMetadataWithLocation {
                        name: Some(name.into()),
                        location: location.into(),
                        description: Some(description.into()),
                    },
                    unit: "°C".into(),
                    ..Default::default()
                },
            );
        }

        self.add_mutator(Mutator {
            mutation: Mutation::TemperatureSensorValue(name.into()),
            topic: topic.into(),
        });

        info!("Added temperature sensor");
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn add_humidity_sensor(
        &self,
        name: &str,
        location: &str,
        description: &str,
        topic: &str,
    ) {
        self.ensure_sensors_struct();

        {
            let mut state = self.state.lock().unwrap();
            state.status.sensors.as_mut().unwrap().humidity.push(
                spaceapi::sensors::HumiditySensor {
                    metadata: spaceapi::sensors::SensorMetadataWithLocation {
                        name: Some(name.into()),
                        location: location.into(),
                        description: Some(description.into()),
                    },
                    unit: "%".into(),
                    ..Default::default()
                },
            );
        }

        self.add_mutator(Mutator {
            mutation: Mutation::HumiditySensorValue(name.into()),
            topic: topic.into(),
        });

        info!("Added humidity sensor");
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Mutator {
    pub(crate) mutation: Mutation,
    pub(crate) topic: String,
}

impl Mutator {
    #[tracing::instrument(skip(status))]
    fn handle_mqtt_message(&self, status: &mut Status, msg: &paho_mqtt::Message) {
        if self.topic == msg.topic() {
            debug!("Found mutator for topic");
            self.mutation.mutate(status, msg);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Mutation {
    TemperatureSensorValue(String),
    HumiditySensorValue(String),
}

impl Mutation {
    #[tracing::instrument(skip(status))]
    fn mutate(&self, status: &mut Status, msg: &paho_mqtt::Message) {
        match self {
            Mutation::TemperatureSensorValue(name) => {
                debug!("Updating temperature sensor");
                if status.sensors.is_some() {
                    match status
                        .sensors
                        .as_mut()
                        .unwrap()
                        .temperature
                        .iter_mut()
                        .find(|s| match &s.metadata.name {
                            None => false,
                            Some(n) => n == name,
                        }) {
                        Some(mut sensor) => match str::parse(&msg.payload_str()) {
                            Ok(v) => {
                                sensor.value = v;
                                info!("Set sensor value to {}", v);
                            }
                            Err(e) => {
                                warn!("Failed to parse string as value ({})", e);
                            }
                        },
                        None => {
                            warn!("Failed to find sensor with name {}", name);
                        }
                    }
                }
            }
            Mutation::HumiditySensorValue(name) => {
                debug!("Updating humidity sensor");
                if status.sensors.is_some() {
                    match status
                        .sensors
                        .as_mut()
                        .unwrap()
                        .humidity
                        .iter_mut()
                        .find(|s| match &s.metadata.name {
                            None => false,
                            Some(n) => n == name,
                        }) {
                        Some(mut sensor) => match str::parse(&msg.payload_str()) {
                            Ok(v) => {
                                sensor.value = v;
                                info!("Set sensor value to {}", v);
                            }
                            Err(e) => {
                                warn!("Failed to parse string as value ({})", e);
                            }
                        },
                        None => {
                            warn!("Failed to find sensor with name {}", name);
                        }
                    }
                }
            }
        }
    }
}
