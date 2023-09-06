use std::{collections::HashMap, sync::Arc};

use sqlx::PgPool;
use tide::sse;
use tokio::sync;

pub type Request = tide::Request<State>;

#[derive(Clone, Debug)]
pub struct State {
    db: PgPool,
    sse: Arc<sync::RwLock<HashMap<i32, Vec<sse::Sender>>>>,
}

impl State {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            sse: Arc::new(sync::RwLock::new(HashMap::new())),
        }
    }

    pub fn db(&self) -> &PgPool {
        &self.db
    }

    pub async fn insert_sse_sender(&self, id: i32, sender: sse::Sender) {
        let mut map = self.sse.write().await;
        map.entry(id).or_default().push(sender);
    }

    pub async fn send_to(&self, id: i32, name: &str, value: &str) -> tide::Result<()> {
        let map = self.sse.read().await;
        let Some(senders) = map.get(&id) else {
            return Ok(());
        };

        for sender in senders {
            sender.send(name, value, None).await?;
        }

        Ok(())
    }
}
