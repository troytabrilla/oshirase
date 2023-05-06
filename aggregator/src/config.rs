use serde::de::DeserializeOwned;
use std::{error::Error, fs::File};

pub trait Conf {
    type Config: DeserializeOwned;

    fn get_config(filename: &str) -> Result<Self::Config, Box<dyn Error>> {
        let file: File = File::open(filename)?;
        let config = serde_yaml::from_reader(&file)?;
        Ok(config)
    }
}
