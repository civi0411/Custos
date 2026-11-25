use rusqlite::Connection;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub const DEFAULT_POOL_MAX_SIZE: usize = 16;

pub struct ReaderPool {
    pub(crate) uri: String,
    pub(crate) is_memory: bool,
    pub(crate) idle: Mutex<Vec<Connection>>,
    pub(crate) max_size: usize,
}

impl ReaderPool {
    pub fn new(uri: String, is_memory: bool, max_size: usize) -> Self {
        Self {
            uri,
            is_memory,
            idle: Mutex::new(Vec::new()),
            max_size,
        }
    }
}

pub struct PooledReader {
    pub(crate) conn: Option<Connection>,
    pub(crate) pool: Arc<ReaderPool>,
}

impl Deref for PooledReader {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.conn
            .as_ref()
            .expect("Connection available in PooledReader")
    }
}

impl Drop for PooledReader {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if let Ok(mut idle) = self.pool.idle.lock() {
                if idle.len() < self.pool.max_size {
                    idle.push(conn);
                }
            }
        }
    }
}
