use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mage_error::Error;
use tokio::sync::RwLock;

use super::*;

pub struct Session {
    id: uuid::Uuid,
    sender: tokio::sync::mpsc::WeakUnboundedSender<ws::Message>,
}

impl Session {
    pub fn id(&self) -> uuid::Uuid {
        self.id
    }

    pub fn send(&self, message: impl Into<ws::Message>) -> Result<(), atp::Error> {
        self.sender
            .upgrade()
            .ok_or(atp::error::socket("inactive socket"))?
            .send(message.into())
            .map_err(atp::error::socket)
    }
}

impl From<tokio::sync::mpsc::WeakUnboundedSender<ws::Message>> for Session {
    fn from(sender: tokio::sync::mpsc::WeakUnboundedSender<ws::Message>) -> Self {
        Self {
            id: uuid::Uuid::now_v7(),
            sender,
        }
    }
}

impl From<tokio::sync::mpsc::UnboundedSender<ws::Message>> for Session {
    fn from(sender: tokio::sync::mpsc::UnboundedSender<ws::Message>) -> Self {
        Self {
            id: uuid::Uuid::now_v7(),
            sender: sender.downgrade(),
        }
    }
}

pub struct Pool {
    cursor: AtomicUsize,
    sessions: Arc<RwLock<Vec<Session>>>,
}

impl Pool {
    pub fn new() -> Self {
        Self {
            cursor: AtomicUsize::new(0),
            sessions: Arc::new(RwLock::new(vec![])),
        }
    }

    pub async fn put(&self, session: impl Into<Session>) {
        let mut guard = self.sessions.write().await;
        guard.push(session.into());
    }

    pub async fn send(&self, message: impl Into<ws::Message>) -> Result<bool, Error> {
        let mut guard = self.sessions.write().await;
        guard.retain(|s| s.sender.upgrade().is_some());

        let mut i = self.cursor.fetch_add(1, Ordering::SeqCst);

        if i >= guard.len() {
            i = 0;
            self.cursor.store(0, Ordering::Relaxed);
        }

        let session = match guard.get(i) {
            None => return Ok(false),
            Some(v) => v,
        };

        let sender = match session.sender.upgrade() {
            None => return Ok(false),
            Some(v) => v,
        };

        sender.send(message.into()).map_err(mage_error::atp)?;
        Ok(true)
    }
}

#[derive(Default, Clone)]
pub struct Connections {
    pools: Arc<RwLock<HashMap<uuid::Uuid, Pool>>>,
}

impl Connections {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, pool_id: uuid::Uuid, session: impl Into<Session>) {
        let mut guard = self.pools.write().await;
        let pool = guard.entry(pool_id).or_insert(Pool::new());
        pool.put(session).await;
    }

    pub async fn send(&self, pool_id: uuid::Uuid, message: impl Into<ws::Message>) -> Result<(), Error> {
        let mut guard = self.pools.write().await;
        let pool = match guard.get(&pool_id) {
            None => return Ok(()),
            Some(pool) => pool,
        };

        match pool.send(message).await {
            Err(err) => {
                guard.remove(&pool_id);
                Err(err)
            }
            Ok(false) => {
                guard.remove(&pool_id);
                Ok(())
            }
            Ok(true) => Ok(()),
        }
    }
}
