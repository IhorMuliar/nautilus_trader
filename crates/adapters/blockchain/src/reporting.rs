// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Performance reporting and metrics tracking for blockchain operations.

use std::{collections::VecDeque, time::{Duration, Instant}};

/// Tracks performance metrics during block synchronization
#[derive(Debug)]
pub struct BlockSyncMetrics {
    start_time: Instant,
    last_progress_time: Instant,
    blocks_processed: u64,
    from_block: u64,
    total_blocks: u64,
    progress_update_interval: u64,
    next_progress_threshold: u64,
}

impl BlockSyncMetrics {
    /// Creates a new metrics tracker for block synchronization
    #[must_use]
    pub fn new(from_block: u64, total_blocks: u64, update_interval: u64) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_progress_time: now,
            blocks_processed: 0,
            from_block,
            total_blocks,
            progress_update_interval: update_interval,
            next_progress_threshold: from_block + update_interval,
        }
    }

    /// Updates metrics after a database operation
    pub const fn update(&mut self, batch_size: usize) {
        self.blocks_processed += batch_size as u64;
    }

    /// Checks if progress should be logged based on the current block number
    #[must_use]
    pub const fn should_log_progress(&self, block_number: u64, current_block: u64) -> bool {
        block_number >= self.next_progress_threshold || block_number >= current_block
    }

    /// Logs current progress with detailed metrics
    pub fn log_progress(&mut self, block_number: u64) {
        let elapsed = self.start_time.elapsed();
        let interval_elapsed = self.last_progress_time.elapsed();
        let interval_blocks =
            if block_number > self.next_progress_threshold - self.progress_update_interval {
                block_number - (self.next_progress_threshold - self.progress_update_interval)
            } else {
                self.progress_update_interval
            };

        // Calculate rates
        let avg_rate = self.blocks_processed as f64 / elapsed.as_secs_f64();
        let current_rate = interval_blocks as f64 / interval_elapsed.as_secs_f64();
        let progress_pct =
            (self.blocks_processed as f64 / self.total_blocks as f64 * 100.0).min(100.0);

        // Estimate remaining time
        let blocks_remaining = self.total_blocks.saturating_sub(self.blocks_processed);
        let eta_display = calculate_eta(blocks_remaining, avg_rate);

        tracing::info!(
            "Block sync progress: {:.1}% | Block: {} | Rate: {:.0} blocks/s | Avg: {:.0} blocks/s | ETA: {}",
            progress_pct,
            block_number,
            current_rate,
            avg_rate,
            eta_display
        );

        self.next_progress_threshold = block_number + self.progress_update_interval;
        self.last_progress_time = Instant::now();
    }

    /// Logs final statistics summary
    pub fn log_final_stats(&self) {
        let total_elapsed = self.start_time.elapsed();
        let avg_rate = self.blocks_processed as f64 / total_elapsed.as_secs_f64();
        tracing::info!(
            "Finished syncing blocks | Total: {} blocks in {:.1}s | Avg rate: {:.0} blocks/s",
            self.blocks_processed,
            total_elapsed.as_secs_f64(),
            avg_rate
        );
    }
}

/// Formats duration into human-readable time (e.g., "3.2h", "45m", "2.5d")
fn format_duration(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{seconds:.0}s")
    } else if seconds < 3600.0 {
        format!("{:.0} minutes", seconds / 60.0)
    } else if seconds < 86400.0 {
        format!("{:.1} hours", seconds / 3600.0)
    } else {
        format!("{:.1} days", seconds / 86400.0)
    }
}

/// Calculates and formats ETA based on rate and remaining work
fn calculate_eta(blocks_remaining: u64, avg_rate: f64) -> String {
    let eta_seconds = blocks_remaining as f64 / avg_rate;
    if eta_seconds < 60.0 {
        format!("{eta_seconds:.0}s")
    } else if eta_seconds < 3600.0 {
        format!("{:.0}m", eta_seconds / 60.0)
    } else if eta_seconds < 86400.0 {
        format!("{:.1}h", eta_seconds / 3600.0)
    } else {
        format!("{:.1}d", eta_seconds / 86400.0)
    }
}

/// Tracks rates over a sliding time window
#[derive(Debug)]
pub struct RateTracker {
    window_size: Duration,
    samples: VecDeque<(Instant, u64)>,
    total_count: u64,
}

impl RateTracker {
    /// Creates a new rate tracker with the specified window size
    #[must_use]
    pub fn new(window_size: Duration) -> Self {
        Self {
            window_size,
            samples: VecDeque::new(),
            total_count: 0,
        }
    }

    /// Records a new sample
    pub fn record(&mut self, count: u64) {
        let now = Instant::now();
        self.samples.push_back((now, count));
        self.total_count += count;
        self.cleanup_old_samples(now);
    }

    /// Gets the current rate (items per second) within the window
    #[must_use]
    pub fn current_rate(&mut self) -> f64 {
        let now = Instant::now();
        self.cleanup_old_samples(now);
        
        if self.samples.is_empty() {
            return 0.0;
        }
        
        let window_total: u64 = self.samples.iter().map(|(_, count)| count).sum();
        let window_duration = now.duration_since(self.samples.front().unwrap().0);
        
        if window_duration.as_secs_f64() > 0.0 {
            window_total as f64 / window_duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Removes samples older than the window size
    fn cleanup_old_samples(&mut self, now: Instant) {
        while let Some((time, _)) = self.samples.front() {
            if now.duration_since(*time) > self.window_size {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }
}

/// Result of a token batch fetch operation
#[derive(Debug, Default)]
pub struct TokenBatchResult {
    pub success_count: usize,
    pub failure_count: usize,
}

/// Result of pool processing operation
#[derive(Debug, Default)]
pub struct PoolProcessingResult {
    pub processed_count: usize,
    pub skipped_count: usize,
}

/// Tracks performance metrics during pool synchronization with streaming support
#[derive(Debug)]
pub struct PoolSyncMetrics {
    // Time tracking
    start_time: Instant,
    last_progress_time: Instant,
    
    // Pool metrics
    pools_discovered: u64,
    pools_processed: u64,
    pools_pending: u64,
    pools_skipped: u64,
    
    // Token metrics  
    unique_tokens_seen: u64,
    tokens_fetched: u64,
    tokens_failed: u64,
    token_batches_sent: u64,
    
    // Progress tracking
    last_logged_pools: u64,
    progress_interval: u64,
    
    // Rate tracking (5-minute windows)
    discovery_rate_tracker: RateTracker,
    processing_rate_tracker: RateTracker,
}

impl PoolSyncMetrics {
    /// Creates a new metrics tracker for pool synchronization
    #[must_use]
    pub fn new(progress_interval: u64) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_progress_time: now,
            pools_discovered: 0,
            pools_processed: 0,
            pools_pending: 0,
            pools_skipped: 0,
            unique_tokens_seen: 0,
            tokens_fetched: 0,
            tokens_failed: 0,
            token_batches_sent: 0,
            last_logged_pools: 0,
            progress_interval,
            discovery_rate_tracker: RateTracker::new(Duration::from_secs(300)), // 5-minute window
            processing_rate_tracker: RateTracker::new(Duration::from_secs(300)),
        }
    }

    /// Records that a pool was discovered
    pub fn record_pool_discovered(&mut self) {
        self.pools_discovered += 1;
        self.pools_pending += 1;
        self.discovery_rate_tracker.record(1);
    }

    /// Records the result of a token batch fetch
    pub fn record_token_batch(
        &mut self,
        unique_tokens: u64,
        success_count: usize,
        failure_count: usize,
    ) {
        self.unique_tokens_seen += unique_tokens;
        self.tokens_fetched += success_count as u64;
        self.tokens_failed += failure_count as u64;
        self.token_batches_sent += 1;
    }

    /// Records pools that were processed
    pub fn record_pools_processed(&mut self, processed: usize, skipped: usize) {
        self.pools_processed += processed as u64;
        self.pools_skipped += skipped as u64;
        self.pools_pending = self.pools_pending.saturating_sub((processed + skipped) as u64);
        self.processing_rate_tracker.record(processed as u64);
    }

    /// Checks if progress should be logged
    #[must_use]
    pub const fn should_log_progress(&self) -> bool {
        self.pools_discovered >= self.last_logged_pools + self.progress_interval
    }

    /// Logs current progress with streaming-aware metrics
    pub fn log_streaming_progress(&mut self, dex_id: &str) {
        let elapsed = self.start_time.elapsed();
        
        // Calculate rates
        let discovery_rate = self.discovery_rate_tracker.current_rate();
        let processing_rate = self.processing_rate_tracker.current_rate();
        let token_success_rate = if self.tokens_fetched + self.tokens_failed > 0 {
            self.tokens_fetched as f64 / (self.tokens_fetched + self.tokens_failed) as f64 * 100.0
        } else {
            0.0
        };
        
        // Estimate completion if we have pending pools
        let completion_estimate = if self.pools_pending > 0 && processing_rate > 0.0 {
            let eta_seconds = self.pools_pending as f64 / processing_rate;
            format!(" | ETA: {}", format_duration(eta_seconds))
        } else {
            String::new()
        };
        
        tracing::info!(
            "[{}] Pools: {} discovered, {} processed, {} pending | \
             Tokens: {} fetched ({:.1}% success) | \
             Rates: {:.0} pools/s discovery, {:.0} pools/s processing{}",
            dex_id,
            self.pools_discovered,
            self.pools_processed,
            self.pools_pending,
            self.tokens_fetched,
            token_success_rate,
            discovery_rate,
            processing_rate,
            completion_estimate
        );
        
        self.last_progress_time = Instant::now();
        self.last_logged_pools = self.pools_discovered;
    }

    /// Logs final statistics summary
    pub fn log_final_summary(&self, dex_id: &str) {
        let total_elapsed = self.start_time.elapsed();
        let avg_discovery_rate = self.pools_discovered as f64 / total_elapsed.as_secs_f64();
        let avg_processing_rate = self.pools_processed as f64 / total_elapsed.as_secs_f64();
        
        let pool_efficiency = if self.pools_discovered > 0 {
            self.pools_processed as f64 / self.pools_discovered as f64 * 100.0
        } else {
            0.0
        };
        
        let token_efficiency = if self.unique_tokens_seen > 0 {
            self.tokens_fetched as f64 / self.unique_tokens_seen as f64 * 100.0
        } else {
            0.0
        };
        
        tracing::info!(
            "[{}] Sync completed in {} | \
             Pools: {} discovered, {} processed, {} skipped | \
             Tokens: {} unique, {} fetched, {} failed ({} batches) | \
             Avg rates: {:.1} pools/s discovery, {:.1} pools/s processing | \
             Efficiency: {:.1}% pools processed, {:.1}% tokens fetched",
            dex_id,
            format_duration(total_elapsed.as_secs_f64()),
            self.pools_discovered,
            self.pools_processed,
            self.pools_skipped,
            self.unique_tokens_seen,
            self.tokens_fetched,
            self.tokens_failed,
            self.token_batches_sent,
            avg_discovery_rate,
            avg_processing_rate,
            pool_efficiency,
            token_efficiency
        );
    }
}
