use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod::configuration::get_configuration;
use zero2prod::issue_delivery_worker::run_worker_until_stopped;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Telemetry setup
    let subscriber = get_subscriber("rs_z2p", "info", std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration");
    let app = Application::build(configuration.clone()).await?;

    tracing::info!("Starting server port: {}", app.port());

    let application_task = tokio::spawn(app.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration));
    tokio::select! {
        app_outcome = application_task => {report_exit("Application API", app_outcome)},
        worker_outcome = worker_task => {report_exit("Issue Delivery Worker", worker_outcome)},
    };
    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(_)) => tracing::info!("{} task completed successfully.", task_name),
        Ok(Err(e)) => {
            tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{} task failed", task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} task failed to complete", task_name
            )
        }
    }
}
