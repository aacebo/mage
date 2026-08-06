use std::env;

use crate::routing;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub rabbitmq_url: String,
    pub routing: routing::Policy,
}

impl Config {
    pub fn from_env() -> mage_error::Result<Self> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .map_err(mage_error::config)?;

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://admin:admin@localhost:5432/main".to_string());
        let rabbitmq_url = env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://admin:admin@localhost:5672".to_string());
        let candidate_limit = parse_env("ROUTING_CANDIDATE_LIMIT", "5")?;
        let min_confidence = parse_env("ROUTING_MIN_CONFIDENCE", "0.20")?;
        let ambiguity_margin = parse_env("ROUTING_AMBIGUITY_MARGIN", "0.05")?;
        let routing = routing::Policy::new(candidate_limit, min_confidence, ambiguity_margin)?;

        Ok(Self {
            port,
            database_url,
            rabbitmq_url,
            routing,
        })
    }
}

fn parse_env<T>(name: &str, default: &str) -> mage_error::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .map_err(|error| mage_error::config(format!("invalid {name}: {error}")))
}
