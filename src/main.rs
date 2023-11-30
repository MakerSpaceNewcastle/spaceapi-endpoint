mod makerspace;
mod metrics;
mod mqtt;
mod mutators;
mod status;
mod utils;

use axum::{routing::get, Router};
use clap::Parser;
use kagiyama::{AlwaysReady, Watcher};
use tokio::task::JoinSet;
use tracing::{error, info};

type Tasks = JoinSet<()>;
type ShutdownSender = tokio::sync::broadcast::Sender<()>;
type ShutdownReceiver = tokio::sync::broadcast::Receiver<()>;

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about)]
struct Cli {
    /// MQTT broker address
    #[clap(value_parser, long, env = "MQTT_BROKER")]
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
        crate::metrics::register_metrics(registry)
    }

    let mut tasks = Tasks::new();
    let (shutdown, mut shutdown_rx) = tokio::sync::broadcast::channel(1);

    let status = crate::makerspace::build_status(&mut tasks, &shutdown, mqtt_client).await;

    let app = Router::new()
        .route(
            "/",
            get({
                let status = status.clone();
                || async move { status.http_get().await }
            }),
        )
        .route(
            "/badge/simple",
            get({
                let status = status.clone();
                || async move { status.http_get_badge_simple().await }
            }),
        )
        .route(
            "/badge",
            get({
                let status = status.clone();
                || async move { status.http_get_badge_full().await }
            }),
        );

    watcher.start_server(args.observability_address).await;

    info!("Starting API server on {}", args.api_address);
    tasks.spawn({
        let mut shutdown_rx = shutdown.subscribe();
        async move {
            axum::Server::bind(&args.api_address)
                .serve(app.into_make_service())
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.recv().await;
                })
                .await
                .expect("server should be started");
        }
    });

    let shutdown_watch_handle = tokio::task::spawn({
        let shutdown = shutdown.clone();
        let shutdown_rx = shutdown.subscribe();
        async move {
            while let Some(res) = tasks.join_next().await {
                info!("Task result = {res:?}");
                match res {
                    Ok(_) => {}
                    Err(_) => {
                        // If a shutdown has not been requested
                        if shutdown_rx.is_empty() {
                            error!("Task failed without shutdown being requested");
                            shutdown.send(()).expect("shutdown signal should be sent");
                        }
                    }
                }
            }
        }
    });

    // Wait for exit signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = shutdown_rx.recv() => {}
    };

    info!("Exiting");
    shutdown.send(()).expect("shutdown signal should be sent");
    shutdown_watch_handle
        .await
        .expect("shutdown watch task should exit cleanly");
}
