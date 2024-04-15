use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod::{
    configuration::get_configuration,
    issue_delivery_worker::run_worker_until_stopped,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration");
    let app = tokio::spawn(Application::build(config.clone()).await.run_until_stopped());
    let worker = tokio::spawn(run_worker_until_stopped(config));

    tokio::select! {
        o = app => report_exit("API", o),
        o = worker => report_exit("Background worker", o),
    };

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{task_name} has exited"),
        Ok(Err(e)) => {
            tracing::error!(eror.cause_chain = ?e, error.message = %e, "{task_name} failed")
        }
        Err(e) => {
            tracing::error!(eror.cause_chain = ?e, error.message = %e, "{task_name} task failed to complete")
        }
    }
}
