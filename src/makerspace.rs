use crate::{status::SpaceStatus, ShutdownSender, Tasks};
use spaceapi::{Contact, Link, Location, State, StatusBuilder};

pub(crate) async fn build_status(
    tasks: &mut Tasks,
    shutdown: &ShutdownSender,
    mqtt_client: mqtt_channel_client::Client,
) -> SpaceStatus {
    let status = SpaceStatus::new(
        tasks,
        shutdown.subscribe(),
        base_status(),
        mqtt_client.clone(),
    )
    .await;
    add_mutators(&status, tasks, shutdown, &mqtt_client).await;
    status
}

fn base_status() -> spaceapi::Status {
    StatusBuilder::v14("Maker Space")
        .logo("http://makerspace.pbworks.com/w/file/fetch/43988924/makerspace_logo.png")
        .url("https://www.makerspace.org.uk/")
        .location(Location {
            address: Some("Maker Space, c/o Orbis Community, Ground Floor, 65 High Street, Gateshead, NE8 2AP".into()),
            lat: 54.9652,
            lon: -1.60233,
            timezone: Some("Europe/London".into())
        })
    .contact(Contact {
        matrix: Some("#makerspace-ncl:matrix.org".into()),
        ml: Some("north-east-makers@googlegroups.com".into()),
        twitter: Some("@maker_space".into()),
        ..Default::default()
    })
    .add_link(Link {
        name: "Maker Space Wiki".into(),
        url: "http://makerspace.pbworks.com".into(),
        ..Default::default()
    })
    .add_link(Link {
        name: "North East Makers mailing list".into(),
        url: "https://groups.google.com/g/north-east-makers".into(),
        ..Default::default()
    })
    .add_project("https://github.com/MakerSpaceNewcastle")
        .state(State {
            open: Some(false),
            ..Default::default()
        })
    .build()
    .expect("basic space status should be created")
}

async fn add_mutators(
    status: &SpaceStatus,
    tasks: &mut Tasks,
    shutdown: &ShutdownSender,
    mqtt_client: &mqtt_channel_client::Client,
) {
    add_temperature_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/10/temperature",
        "Ground Floor - Main Space",
    )
    .await;

    add_humidity_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/10/humidity",
        "Ground Floor - Main Space",
    )
    .await;

    add_temperature_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/11/temperature",
        "Basement - Near Workbee CNC",
    )
    .await;

    add_humidity_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/11/humidity",
        "Basement - Near Workbee CNC",
    )
    .await;

    add_temperature_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/12/temperature",
        "Basement - Opposite wall to Workbee CNC",
    )
    .await;

    add_humidity_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/12/humidity",
        "Basement - Opposite wall to Workbee CNC",
    )
    .await;

    add_temperature_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/13/temperature",
        "Basement - Near Bandsaw",
    )
    .await;

    add_humidity_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/13/humidity",
        "Basement - Near Bandsaw",
    )
    .await;

    add_temperature_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/14/temperature",
        "Basement - Near Wood Store",
    )
    .await;

    add_humidity_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/14/humidity",
        "Basement - Near Wood Store",
    )
    .await;

    add_temperature_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/15/temperature",
        "Basement - Inside Old Barrel Drop",
    )
    .await;

    add_humidity_sensor(
        status,
        tasks,
        shutdown,
        mqtt_client.clone(),
        "makerspace/sensors/15/humidity",
        "Basement - Inside Old Barrel Drop",
    )
    .await;
}

async fn add_temperature_sensor(
    status: &SpaceStatus,
    tasks: &mut Tasks,
    shutdown: &ShutdownSender,
    mqtt_client: mqtt_channel_client::Client,
    topic: &str,
    location: &str,
) {
    status
        .add_mutator(Box::new(crate::mutators::MqttTemperature::new(
            tasks,
            shutdown.subscribe(),
            status.mutator_data_notification().await,
            mqtt_client.clone(),
            spaceapi::sensors::TemperatureSensorTemplate {
                metadata: spaceapi::sensors::SensorMetadataWithLocation {
                    location: location.to_string(),
                    ..Default::default()
                },
                unit: "°C".to_string(),
            },
            topic.to_string(),
        )))
        .await;
}

async fn add_humidity_sensor(
    status: &SpaceStatus,
    tasks: &mut Tasks,
    shutdown: &ShutdownSender,
    mqtt_client: mqtt_channel_client::Client,
    topic: &str,
    location: &str,
) {
    status
        .add_mutator(Box::new(crate::mutators::MqttHumidity::new(
            tasks,
            shutdown.subscribe(),
            status.mutator_data_notification().await,
            mqtt_client.clone(),
            spaceapi::sensors::HumiditySensorTemplate {
                metadata: spaceapi::sensors::SensorMetadataWithLocation {
                    location: location.to_string(),
                    ..Default::default()
                },
                unit: "%".to_string(),
            },
            topic.to_string(),
        )))
        .await;
}
