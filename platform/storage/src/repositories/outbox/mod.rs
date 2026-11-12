use crate::connection::DbConnection;

mod effect;
mod entry;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct OutboxRepository {
    pub(crate) db: DbConnection,
}

impl OutboxRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
}

