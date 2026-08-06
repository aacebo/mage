pub mod http;

use chrono::{DateTime, Utc};
use mage_storage::Storage;
use sqlx::PgPool;

use crate::Config;
use crate::ws::Connections;

#[derive(Clone)]
pub struct Session {
    pool: PgPool,
    amqp: mage_amqp::Socket,
    config: Config,
    connections: Connections,
    started_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn new(pool: PgPool, amqp: mage_amqp::Socket, config: Config) -> Self {
        Self {
            pool,
            amqp,
            config,
            connections: Connections::new(),
            started_at: chrono::Utc::now(),
        }
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn storage(&self) -> Storage<'_> {
        Storage::new(&self.pool)
    }

    pub fn amqp(&self) -> &mage_amqp::Socket {
        &self.amqp
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn connections(&self) -> &Connections {
        &self.connections
    }
}
