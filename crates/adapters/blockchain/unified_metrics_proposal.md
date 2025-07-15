# Unified Blockchain Metrics System

## Overview

Currently, we have two separate metrics systems:
- `BlockSyncMetrics` - for sequential block processing
- `PoolSyncMetrics` - for event-driven pool/token processing

Both are fundamentally tracking blockchain data processing over time. This proposal presents a unified metrics system that can handle both use cases and more.

## Analysis of Current Systems

### Common Patterns
- **Time tracking**: Start time, elapsed time, progress intervals
- **Rate calculation**: Items processed per second
- **Progress reporting**: Periodic updates with ETA
- **Completion tracking**: Final summary statistics

### Key Differences
| Aspect | BlockSyncMetrics | PoolSyncMetrics |
|--------|------------------|-----------------|
| Data source | Sequential blocks | Event streams |
| Processing stages | Single stage | Multi-stage (pools → tokens → processing) |
| Total knowledge | Known upfront | Discovered incrementally |
| Dependencies | None | Pools depend on tokens |

## Proposed Solution: Unified Blockchain Metrics

### Core Design

```rust
/// Unified blockchain metrics that handles both sequential and event-driven processing
#[derive(Debug)]
pub struct UnifiedBlockchainMetrics {
    // Identity
    operation_name: String,
    operation_type: OperationType,
    
    // Time tracking
    start_time: Instant,
    last_log_time: Instant,
    
    // Block context
    from_block: u64,
    to_block: Option<u64>,
    current_block: u64,
    
    // Flexible metric tracking
    metrics: HashMap<String, ItemMetrics>,
    
    // Progress config
    log_interval: Duration,
    log_threshold: u64,
}

#[derive(Debug)]
enum OperationType {
    BlockSync,           // Linear block processing
    EventSync {          // Event-driven processing
        primary_event: String,
        stages: Vec<String>,
    },
    Hybrid,             // Both blocks and events
}
```

### Flexible Item Metrics

```rust
#[derive(Debug, Default)]
struct ItemMetrics {
    // Core metrics
    discovered: u64,
    processed: u64,
    failed: u64,
    pending: u64,
    rate_tracker: RateTracker,
    
    // Optional fields for specific use cases
    batches_sent: Option<u64>,
    success_rate: Option<f64>,
    dependencies_met: Option<u64>,
}
```

## Usage Examples

### Block Sync
```rust
let mut metrics = UnifiedBlockchainMetrics::new_block_sync(0, 1_000_000);
metrics.record("blocks", MetricEvent::Processed(1000));
// Output: [Block Sync] Blocks: 1000/1000000 (0.1%) | Rate: 250 blocks/s | ETA: 1.1h
```

### Pool Sync
```rust
let mut metrics = UnifiedBlockchainMetrics::new_pool_sync("UniswapV3", 0);
metrics.record("pools", MetricEvent::Discovered(100));
metrics.record("tokens", MetricEvent::BatchProcessed { success: 180, failed: 20 });
metrics.record("pools", MetricEvent::Processed(85));
// Output: [UniswapV3 Pool Sync] Pools: 100 discovered, 85 processed | Tokens: 180 fetched (90% success) | Rates: 10 pools/s discovery, 8.5 pools/s processing
```

### Custom Metrics
```rust
// NFT collection sync
let mut metrics = UnifiedBlockchainMetrics::new("NFT Sync", OperationType::Hybrid, 0, None);
metrics.add_metric("collections");
metrics.add_metric("tokens");
metrics.add_metric("metadata");
```

## Implementation Details

### 1. Metric Events
```rust
pub enum MetricEvent {
    Discovered(u64),
    Processed(u64),
    Failed(u64),
    BatchProcessed { success: u64, failed: u64 },
    BlockReached(u64),
}
```

### 2. Progress Logging
```rust
impl UnifiedBlockchainMetrics {
    pub fn log_progress(&mut self) {
        match &self.operation_type {
            OperationType::BlockSync => self.log_sequential_progress(),
            OperationType::EventSync { .. } => self.log_event_progress(),
            OperationType::Hybrid => self.log_hybrid_progress(),
        }
    }
    
    fn log_sequential_progress(&self) {
        // Format: [Name] Items: X/Y (Z%) | Rate: R/s | ETA: T
    }
    
    fn log_event_progress(&self) {
        // Format: [Name] Stage1: X discovered | Stage2: Y processed | Rates: R1/s, R2/s
    }
}
```

### 3. Builder Pattern for Easy Construction
```rust
impl UnifiedBlockchainMetrics {
    pub fn builder(name: &str) -> MetricsBuilder {
        MetricsBuilder::new(name)
    }
}

pub struct MetricsBuilder {
    name: String,
    operation_type: OperationType,
    metrics: Vec<String>,
}

// Usage:
let metrics = UnifiedBlockchainMetrics::builder("DeFi Sync")
    .operation_type(OperationType::EventSync { 
        primary: "swaps", 
        stages: vec!["pools", "tokens"] 
    })
    .add_metric("swaps")
    .add_metric("pools")
    .add_metric("tokens")
    .log_interval(Duration::from_secs(30))
    .build();
```

## Migration Path

### Phase 1: Add Unified System
1. Implement `UnifiedBlockchainMetrics` alongside existing systems
2. Add compatibility methods that match existing APIs

### Phase 2: Gradual Migration
```rust
// Old
let metrics = BlockSyncMetrics::new(from, total, interval);

// New (with compatibility wrapper)
let metrics = UnifiedBlockchainMetrics::new_block_sync(from, to);
```

### Phase 3: Deprecate Old Systems
1. Mark old metrics as deprecated
2. Update all usages to unified system
3. Remove old implementations

## Benefits

### 1. **Consistency**
- Same logging format across all blockchain operations
- Unified progress reporting interface
- Consistent rate tracking and ETA calculations

### 2. **Flexibility**
- Supports any number of processing stages
- Optional metrics for specific use cases
- Can mix sequential and event-driven processing

### 3. **Extensibility**
- Easy to add new metric types
- Custom operation types for special cases
- Plugin architecture for specialized formatting

### 4. **Performance**
- Shared rate tracking infrastructure
- Efficient memory usage with optional fields
- Batched metric updates

### 5. **Observability**
- Structured metrics for monitoring systems
- Export to Prometheus/OpenTelemetry
- Correlation across different operations

## Example Output Formats

### Sequential Processing
```
[Block Sync] Blocks: 500,000/1,000,000 (50.0%) | Rate: 1,250 blocks/s | Avg: 1,000 blocks/s | ETA: 6m 40s
```

### Event-Driven Processing
```
[UniswapV3 Pool Sync] Pools: 45,231 discovered, 42,156 processed, 3,075 pending | Tokens: 8,234 fetched (92.3% success) | Rates: 125 pools/s discovery, 105 pools/s processing | ETA: 29s
```

### Hybrid Processing
```
[NFT Marketplace Sync] Block: 18,500,000 | Collections: 1,234 discovered | Tokens: 456,789 processed | Metadata: 89.5% cached | Rates: 50 collections/s, 2,500 tokens/s
```

## Future Enhancements

1. **Real-time Dashboards**
   - WebSocket streaming of metrics
   - Live progress visualization
   - Historical performance tracking

2. **Adaptive Behavior**
   - Dynamic batch sizing based on performance
   - Automatic rate limiting detection
   - Resource usage optimization

3. **Integration Points**
   - OpenTelemetry spans for distributed tracing
   - Prometheus metrics export
   - CloudWatch/Datadog integration

4. **Advanced Analytics**
   - Performance regression detection
   - Bottleneck identification
   - Predictive completion times