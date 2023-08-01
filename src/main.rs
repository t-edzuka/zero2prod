use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Telemetry setup
    let subscriber = get_subscriber("rs_z2p", "info", std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration");
    let app = Application::build(configuration.clone()).await?;

    tracing::info!("Starting server port: {}", app.port());
    app.run_until_stopped().await
}
