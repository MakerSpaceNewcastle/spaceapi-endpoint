use super::{DataNotificationSender, Mutator};
use crate::{utils::SpaceapiStatusExt, ShutdownReceiver, Tasks};
use spaceapi::{
    sensors::{HumiditySensorTemplate, SensorTemplate, TemperatureSensorTemplate},
    Status,
};
use std::sync::{Arc, Mutex};
use tracing::warn;

pub(crate) struct MqttSensor<T> {
    template: T,
    value_str: Arc<Mutex<Option<String>>>,
}

impl<T> MqttSensor<T> {
    pub(crate) fn new(
        tasks: &mut Tasks,
        mut shutdown: ShutdownReceiver,
        data_notification: DataNotificationSender,
        mqtt_client: mqtt_channel_client::Client,
        template: T,
        value_topic: String,
    ) -> Self {
        mqtt_client.subscribe(
            mqtt_channel_client::SubscriptionBuilder::default()
                .topic(value_topic.clone())
                .build()
                .expect("value topic subscription should be valid"),
        );

        let value_str = Arc::new(Mutex::new(None));

        let mut mqtt_rx = mqtt_client.rx_channel();

        tasks.spawn({
            let value_str = value_str.clone();

            async move {
                loop {
                    tokio::select! {
                        event = shutdown.recv() => {
                            if event.is_ok() {
                                break;
                            }
                        }
                        event = mqtt_rx.recv() => {
                            if let Ok(mqtt_channel_client::Event::Rx(msg)) = event {
                                if msg.topic() == value_topic {
                                    let _ = value_str.lock().unwrap().insert(msg.payload_str().to_string());
                                    if let Err(e) = data_notification.send(()).await {
                                        warn!("Failed notify of new data (error = {e})");
                                    }
                                }
                            }
                        }
                    };
                }
            }
        });

        Self {
            template,
            value_str,
        }
    }
}

impl<T: SensorTemplate + Send> Mutator for MqttSensor<T> {
    fn apply(&self, status: &mut Status) {
        if let Some(value) = self.value_str.lock().unwrap().as_ref() {
            status.ensure_sensors_struct_exists();
            self.template
                .to_sensor(value, status.sensors.as_mut().unwrap());
        }
    }
}

pub(crate) type MqttHumidity = MqttSensor<HumiditySensorTemplate>;
pub(crate) type MqttTemperature = MqttSensor<TemperatureSensorTemplate>;
