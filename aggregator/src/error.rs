use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

#[derive(Debug)]
pub struct CustomError {
    message: String,
}

impl CustomError {
    pub fn new(message: &str) -> CustomError {
        CustomError {
            message: message.to_owned(),
        }
    }

    pub fn boxed(message: &str) -> Box<CustomError> {
        Box::new(CustomError::new(message))
    }
}

impl Display for CustomError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CustomError {}
