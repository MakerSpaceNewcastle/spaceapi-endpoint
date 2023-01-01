use std::time::Duration;
use tracing::info;

#[tracing::instrument(skip(password))]
pub(crate) async fn create_client(broker: url::Url, password: &str) -> mqtt_channel_client::Client {
    info!("Creating client");
    let mqtt_client = mqtt_channel_client::Client::new(
        paho_mqtt::create_options::CreateOptionsBuilder::new()
            .server_uri(broker)
            .persistence(paho_mqtt::PersistenceType::None)
            .finalize(),
        mqtt_channel_client::ClientConfig::default(),
    )
    .unwrap();

    info!("Logging in");
    mqtt_client
        .start(
            paho_mqtt::connect_options::ConnectOptionsBuilder::new()
                .clean_session(true)
                .automatic_reconnect(Duration::from_secs(1), Duration::from_secs(5))
                .keep_alive_interval(Duration::from_secs(5))
                .user_name("dan")
                .password(password)
                .finalize(),
        )
        .await
        .unwrap();

    mqtt_client
}
