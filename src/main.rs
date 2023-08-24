use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod::configuration::get_configuration;
use zero2prod::idempotency_expiring_worker;
use zero2prod::issue_delivery_worker;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    const EXPIRED_AFTER_HOUR: u8 = 48;
    // Telemetry setup
    let subscriber = get_subscriber("rs_z2p", "info", std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration");
    let app = Application::build(configuration.clone()).await?;

    tracing::info!("Starting server port: {}", app.port());

    let application_task = tokio::spawn(app.run_until_stopped());
    let sending_email_task = tokio::spawn(issue_delivery_worker::run_worker_until_stopped(
        configuration.clone(),
    ));
    let removing_expired_idempotency_key_task = tokio::spawn(
        idempotency_expiring_worker::run_until_worker_stopped(configuration, EXPIRED_AFTER_HOUR),
    );
    tokio::select! {
        app_outcome = application_task => {report_exit("Application API", app_outcome)},
        email_sending_worker_outcome = sending_email_task => {
            report_exit("Issue Delivery Worker", email_sending_worker_outcome)
        },
        removing_expired_idempotency_key_outcome = removing_expired_idempotency_key_task => {
            report_exit("Idempotency Expiring Worker", removing_expired_idempotency_key_outcome)
        },
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
