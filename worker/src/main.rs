fn main() -> Result<(), worker::WorkerError> {
    worker::run()?;

    Ok(())
}
