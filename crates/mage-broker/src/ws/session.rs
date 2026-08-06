use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mage_error::Error;
use tokio::sync::RwLock;

use super::*;

type Sender = tokio::sync::mpsc::WeakUnboundedSender<ws::Message>;

pub struct Pool {
    cursor: AtomicUsize,
    sessions: Arc<RwLock<Vec<Sender>>>,
}

impl Pool {
    pub fn new() -> Self {
        Self {
            cursor: AtomicUsize::new(0),
            sessions: Arc::new(RwLock::new(vec![])),
        }
    }

    pub async fn put(&self, session: impl Into<Sender>) {
        let mut guard = self.sessions.write().await;
        guard.push(session.into());
    }

    pub async fn send(&self, message: impl Into<ws::Message>) -> Result<bool, Error> {
        let mut guard = self.sessions.write().await;
        guard.retain(|s| s.upgrade().is_some());

        let mut i = self.cursor.fetch_add(1, Ordering::SeqCst);

        if i >= guard.len() {
            i = 0;
            self.cursor.store(0, Ordering::Relaxed);
        }

        let session = match guard.get(i) {
            None => return Ok(false),
            Some(v) => v,
        };

        let sender = match session.upgrade() {
            None => return Ok(false),
            Some(v) => v,
        };

        sender.send(message.into()).map_err(mage_error::internal)?;
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

    pub async fn register(&self, pool_id: uuid::Uuid, session: impl Into<Sender>) {
        let mut guard = self.pools.write().await;
        let pool = guard.entry(pool_id).or_insert(Pool::new());
        pool.put(session).await;
    }

    #[allow(unused)]
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
