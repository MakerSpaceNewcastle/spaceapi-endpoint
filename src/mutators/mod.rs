mod mqtt_sensor;

use spaceapi::Status;
use std::time::Duration;

pub(crate) use self::mqtt_sensor::{MqttHumidity, MqttTemperature};

pub(crate) static UPDATE_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) type DataNotificationSender = tokio::sync::mpsc::Sender<()>;

pub(crate) trait Mutator: Send {
    fn apply(&self, status: &mut Status);
}
