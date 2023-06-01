mod error;

pub use error::WorkerError;

/**
 * TODO Pivoting to a general worker queue paradigm instead of specifically an aggregator.
 * This should be able to handle any background task that doesn't need to be performed on the
 * `api` or `ui`. For example, generating alt titles in the background to reduce title processing
 * on the server. Accept jobs through some message passing system, i.e. Redis, RabbitMQ, Kafka, etc.
 * Could potentially even provide a REST API, but that should be considered a stretch goal.
 */
pub fn run() -> Result<(), WorkerError> {
    Ok(())
}
