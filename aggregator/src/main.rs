fn main() -> Result<(), aggregator::AggregatorError> {
    aggregator::run()?;

    Ok(())
}
