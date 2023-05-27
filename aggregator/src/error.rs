#[derive(Debug)]
pub struct AggregatorError {
    message: String,
}

impl AggregatorError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl std::error::Error for AggregatorError {}

impl std::fmt::Display for AggregatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Aggregator Error: {}", self.message)
    }
}
