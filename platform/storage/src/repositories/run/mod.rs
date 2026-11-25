mod claims;
mod launches;
mod runs;
mod telemetry;
mod telemetry_queries;
mod workers;

#[cfg(test)]
mod tests;

use crate::connection::DbConnection;

#[derive(Clone)]
pub struct RunRepository {
    pub(crate) db: DbConnection,
}

impl RunRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
}
