mod metrics;
mod mqtt;
mod status;

use axum::{routing::get, Router};
use clap::Parser;
use kagiyama::{AlwaysReady, Watcher};
use spaceapi::{Contact, Link, Location, State, StatusBuilder};
use tracing::info;

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about)]
struct Cli {
    /// MQTT broker address
    #[clap(
        value_parser,
        long,
        env = "MQTT_BROKER",
        default_value = "tcp://mqtt.makerspace.dan-nixon.com:1883"
    )]
    mqtt_broker: url::Url,

    /// MQTT password
    #[clap(value_parser, long, env = "MQTT_PASSWORD")]
    mqtt_password: String,

    /// Address to listen on for SpaceAPI endpoint
    #[clap(
        value_parser,
        long,
        env = "API_ADDRESS",
        default_value = "127.0.0.1:8080"
    )]
    api_address: std::net::SocketAddr,

    /// Address to listen on for observability/metrics endpoints
    #[clap(
        value_parser,
        long,
        env = "OBSERVABILITY_ADDRESS",
        default_value = "127.0.0.1:9090"
    )]
    observability_address: std::net::SocketAddr,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    let mqtt_client = mqtt::create_client(args.mqtt_broker, &args.mqtt_password).await;

    let mut watcher = Watcher::<AlwaysReady>::default();
    {
        let mut registry = watcher.metrics_registry();
        let registry = registry.sub_registry_with_prefix("spaceapi");
        mqtt_client.register_metrics(registry);
        registry.register("requests", "SpaceAPI requests", metrics::REQUESTS.clone());
    }

    let status =
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
            ..State::default()
        })
        .build()
        .expect("basic space status should be created");

    let status = status::SpaceStatus::new(status, mqtt_client);

    status.add_temperature_sensor(
        "Main Space",
        "Ground Floor - Main Space",
        "West wall under windows",
        "makerspace/sensors/10/temperature",
    );
    status.add_humidity_sensor(
        "Main Space",
        "Ground Floor - Main Space",
        "West wall under windows",
        "makerspace/sensors/10/humidity",
    );

    status.add_temperature_sensor(
        "Workbee",
        "Basement - Near Workbee CNC",
        "",
        "makerspace/sensors/11/temperature",
    );
    status.add_humidity_sensor(
        "Workbee",
        "Basement - Near Workbee CNC",
        "",
        "makerspace/sensors/11/humidity",
    );

    status.add_temperature_sensor(
        "Opposite Workbee",
        "Basement - Opposite Wall to Workbee CNC",
        "",
        "makerspace/sensors/12/temperature",
    );
    status.add_humidity_sensor(
        "Opposite Workbee",
        "Basement - Opposite Wall to Workbee CNC",
        "",
        "makerspace/sensors/12/humidity",
    );

    status.add_temperature_sensor(
        "Bandsaw",
        "Basement - Near Bandsaw",
        "",
        "makerspace/sensors/13/temperature",
    );
    status.add_humidity_sensor(
        "Bandsaw",
        "Basement - Near Bandsaw",
        "",
        "makerspace/sensors/13/humidity",
    );

    status.add_temperature_sensor(
        "Wood Store",
        "Basement - Near Wood Store",
        "",
        "makerspace/sensors/14/temperature",
    );
    status.add_humidity_sensor(
        "Wood Store",
        "Basement - Near Wood Store",
        "",
        "makerspace/sensors/14/humidity",
    );

    status.add_temperature_sensor(
        "Old Barrel Drop",
        "Basement - Inside Old Barrel Drop",
        "",
        "makerspace/sensors/15/temperature",
    );
    status.add_humidity_sensor(
        "Old Barrel Drop",
        "Basement - Inside Old Barrel Drop",
        "",
        "makerspace/sensors/15/humidity",
    );

    let app = Router::new().route("/", get(move || async move { status.http_get() }));

    watcher.start_server(args.observability_address).await;

    info!("Starting API server on {}", args.api_address);
    axum::Server::bind(&args.api_address)
        .serve(app.into_make_service())
        .await
        .expect("API server should be running");
}
