use config::{Config, ConfigError, Environment};
use kafka_settings::KafkaSettings;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub kafka: KafkaSettings,
    pub app: AppSettings,
}

#[derive(Debug, Deserialize)]
pub struct AppSettings {
    pub num_stocks: usize,
    pub initial_equity: Decimal,
    #[serde(deserialize_with = "vec_from_str")]
    pub tickers: Vec<String>,
}

pub fn vec_from_str<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.split(',').map(From::from).collect())
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(Environment::new().separator("__"))?;
        s.try_into()
    }
}
