use std::env;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub rabbitmq_url: String,
    pub console: ConsoleConfig,
}

impl Config {
    pub fn from_env() -> mage_error::Result<Self> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .map_err(mage_error::parse)?;

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://admin:admin@localhost:5432/main".to_string());
        let rabbitmq_url = env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://admin:admin@localhost:5672".to_string());
        let console = ConsoleConfig::from_env()?;

        Ok(Self {
            port,
            database_url,
            rabbitmq_url,
            console,
        })
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ConsoleConfig {
    pub enabled: bool,
    pub tenant_id: Option<uuid::Uuid>,
}

impl ConsoleConfig {
    fn from_env() -> mage_error::Result<Self> {
        let enabled = env::var("CONSOLE_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .map_err(|error| mage_error::config(format!("invalid CONSOLE_ENABLED: {error}")))?;

        let tenant_id = env::var("CONSOLE_TENANT_ID")
            .ok()
            .map(|value| {
                value
                    .parse()
                    .map_err(|error| mage_error::config(format!("invalid CONSOLE_TENANT_ID: {error}")))
            })
            .transpose()?;

        if enabled && tenant_id.is_none() {
            return Err(mage_error::config("CONSOLE_TENANT_ID is required when CONSOLE_ENABLED=true"));
        }

        Ok(Self { enabled, tenant_id })
    }
}
