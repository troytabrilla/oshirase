#[derive(Debug)]
pub struct WorkerError {
    message: String,
}

impl WorkerError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl std::error::Error for WorkerError {}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Worker Error: {}", self.message)
    }
}
