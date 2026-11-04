//! Sovereign Single-Writer Micro-Batching Engine
//!
//! Collects write commands across all 4 Workbenches via an asynchronous MPSC queue
//! and flushes them in batched SQLite transactions every 100ms or 500 items.
//! This eliminates write lock contention, guarantees high throughput, and preserves
//! the Single Sovereign Writer WAL architecture of Custos.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use custos_domain::{DomainError, OhlcvBar};
use rusqlite::params;
use tokio::sync::mpsc;
use crate::connection::DbConnection;

/// Canonical items that can be micro-batched into the database.
#[derive(Debug, Clone)]
pub enum BatchWriteItem {
    /// Quant: Ingestion of OHLCV time-series market bars.
    MarketBars {
        symbol: String,
        timeframe: String,
        bars: Vec<OhlcvBar>,
    },
    /// Engineering / Copilot: Ingestion of command ledger execution entries.
    CommandLedger {
        command_id: String,
        method: String,
        payload_digest: String,
        result_json: String,
        created_at: String,
        expires_at: String,
    },
    /// Research: Insertion of research source citation records.
    ResearchSource {
        source_id: String,
        source_type: String,
        title: String,
        authors_json: String,
        content_hash: String,
        created_at: i64,
    },
}

/// Statistics for observability of the batch writer engine.
#[derive(Debug, Default)]
pub struct BatchWriterStats {
    pub total_items_queued: AtomicU64,
    pub total_items_written: AtomicU64,
    pub batches_committed: AtomicU64,
    pub last_flush_epoch_ms: AtomicU64,
}

/// Cloneable handle to submit write items to the single sovereign background writer.
#[derive(Clone)]
pub struct BatchWriterHandle {
    sender: mpsc::Sender<BatchWriteItem>,
    stats: Arc<BatchWriterStats>,
}

impl BatchWriterHandle {
    /// Submit a write item asynchronously with backpressure.
    pub async fn submit(&self, item: BatchWriteItem) -> Result<(), DomainError> {
        self.stats.total_items_queued.fetch_add(1, Ordering::Relaxed);
        self.sender
            .send(item)
            .await
            .map_err(|e| DomainError::Validation(format!("Failed to enqueue batch write: {}", e)))
    }

    /// Try submitting a write item without waiting. Fails if the channel is full.
    pub fn try_submit(&self, item: BatchWriteItem) -> Result<(), DomainError> {
        self.stats.total_items_queued.fetch_add(1, Ordering::Relaxed);
        self.sender
            .try_send(item)
            .map_err(|e| DomainError::Validation(format!("Batch write buffer full: {}", e)))
    }

    /// Access current observability statistics.
    pub fn stats(&self) -> &BatchWriterStats {
        &self.stats
    }
}

/// Start the background single-writer task with the given parameters.
pub fn spawn_batch_writer(
    db: DbConnection,
    buffer_capacity: usize,
    max_batch_size: usize,
    flush_interval: Duration,
) -> (BatchWriterHandle, tokio::task::JoinHandle<()>) {
    let (sender, mut receiver) = mpsc::channel(buffer_capacity.max(100));
    let stats = Arc::new(BatchWriterStats::default());
    let worker_stats = stats.clone();

    let handle = BatchWriterHandle { sender, stats };

    let join_handle = tokio::spawn(async move {
        let mut buffer: Vec<BatchWriteItem> = Vec::with_capacity(max_batch_size);
        let mut interval = tokio::time::interval(flush_interval);

        loop {
            tokio::select! {
                biased;

                maybe_item = receiver.recv() => {
                    match maybe_item {
                        Some(item) => {
                            buffer.push(item);
                            if buffer.len() >= max_batch_size {
                                flush_buffer(&db, &mut buffer, &worker_stats);
                            }
                        }
                        None => {
                            // Sender channel closed, flush any remaining and terminate cleanly
                            if !buffer.is_empty() {
                                flush_buffer(&db, &mut buffer, &worker_stats);
                            }
                            break;
                        }
                    }
                }

                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        flush_buffer(&db, &mut buffer, &worker_stats);
                    }
                }
            }
        }
    });

    (handle, join_handle)
}

/// Flush buffered items in a single immediate SQLite transaction.
fn flush_buffer(db: &DbConnection, buffer: &mut Vec<BatchWriteItem>, stats: &BatchWriterStats) {
    if buffer.is_empty() {
        return;
    }

    let batch_count = buffer.len();
    let mut total_records = 0u64;

    let res: Result<(), DomainError> = (|| {
        let mut conn = db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(format!("Failed to begin batch tx: {}", e)))?;

        {
            // Prepare statements on demand inside transaction
            let mut bar_stmt = tx.prepare(
                "INSERT OR REPLACE INTO quant_market_bars (symbol, timeframe, timestamp, open, high, low, close, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
            ).map_err(|e| DomainError::Validation(e.to_string()))?;

            let mut cmd_stmt = tx.prepare(
                "INSERT OR REPLACE INTO command_ledger (command_id, method, payload_digest, result_json, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            ).map_err(|e| DomainError::Validation(e.to_string()))?;

            let mut source_stmt = tx.prepare(
                "INSERT OR REPLACE INTO research_sources (id, source_type, title, authors_json, content_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            ).map_err(|e| DomainError::Validation(e.to_string()))?;

            for item in buffer.drain(..) {
                match item {
                    BatchWriteItem::MarketBars { symbol, timeframe, bars } => {
                        for bar in bars {
                            bar_stmt.execute(params![
                                symbol,
                                timeframe,
                                bar.timestamp,
                                bar.open,
                                bar.high,
                                bar.low,
                                bar.close,
                                bar.volume,
                            ]).map_err(|e| DomainError::Validation(e.to_string()))?;
                            total_records += 1;
                        }
                    }
                    BatchWriteItem::CommandLedger { command_id, method, payload_digest, result_json, created_at, expires_at } => {
                        cmd_stmt.execute(params![
                            command_id,
                            method,
                            payload_digest,
                            result_json,
                            created_at,
                            expires_at,
                        ]).map_err(|e| DomainError::Validation(e.to_string()))?;
                        total_records += 1;
                    }
                    BatchWriteItem::ResearchSource { source_id, source_type, title, authors_json, content_hash, created_at } => {
                        source_stmt.execute(params![
                            source_id,
                            source_type,
                            title,
                            authors_json,
                            content_hash,
                            created_at,
                        ]).map_err(|e| DomainError::Validation(e.to_string()))?;
                        total_records += 1;
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| DomainError::Validation(format!("Failed to commit batch tx: {}", e)))?;
        Ok(())
    })();

    if let Err(e) = res {
        eprintln!("[BatchWriter] Flush failure ({} items dropped): {}", batch_count, e);
    } else {
        stats.batches_committed.fetch_add(1, Ordering::Relaxed);
        stats.total_items_written.fetch_add(total_records, Ordering::Relaxed);
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        stats.last_flush_epoch_ms.store(now_ms, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::OhlcvBar;

    #[tokio::test]
    async fn test_batch_writer_lifecycle_and_micro_batching() {
        let db = DbConnection::open_in_memory().expect("open shared memory sqlite");
        let (writer, join_handle) = spawn_batch_writer(
            db.clone(),
            1000,
            10, // flush at 10 items
            Duration::from_millis(50),
        );

        // Submit 25 market bars in 5 batches
        for i in 0..5 {
            let bars = vec![
                OhlcvBar::new(1000 + i * 60, 100.0, 102.0, 99.0, 101.0, 50.0),
                OhlcvBar::new(1000 + i * 60 + 30, 101.0, 103.0, 100.5, 102.5, 60.0),
            ];
            writer
                .submit(BatchWriteItem::MarketBars {
                    symbol: "BTC/USDT".to_string(),
                    timeframe: "1m".to_string(),
                    bars,
                })
                .await
                .expect("submit succeeds");
        }

        // Wait for micro-batching flush interval
        tokio::time::sleep(Duration::from_millis(150)).await;

        let stats = writer.stats();
        assert_eq!(stats.total_items_queued.load(Ordering::Relaxed), 5);
        assert_eq!(stats.total_items_written.load(Ordering::Relaxed), 10);
        assert!(stats.batches_committed.load(Ordering::Relaxed) >= 1);

        // Verify data was written to SQLite
        {
            let conn = db.lock().expect("lock writer");
            let mut stmt = conn
                .prepare("SELECT COUNT(*) FROM quant_market_bars WHERE symbol = 'BTC/USDT'")
                .expect("prepare select");
            let count: i64 = stmt.query_row([], |r| r.get(0)).expect("query count");
            assert_eq!(count, 10);
        }

        // Drop writer to signal termination
        drop(writer);
        join_handle.await.expect("worker task terminates cleanly");
    }
}
