use std::sync::Arc;

use db::Db;
use serenity::futures::lock::Mutex;

use crate::error::Result;

pub type UserId = u64;
pub type ChannelId = u64;
pub type ServerId = u64;

pub struct Database(Arc<Mutex<Db>>);

impl Database {
    pub async fn transaction<T>(&self, f: impl FnOnce(&Db) -> Result<T>) -> Result<T> {
	let db = self.0.lock().await;
	f(&db)
    }
}

impl From<Db> for Database {
    fn from(db: Db) -> Self {
	let db = Arc::new(Mutex::new(db));
	Database(db)
    }
}
