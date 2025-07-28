# Binance Trade WebSocket Client

High-performance WebSocket client for Binance Trade API integration with Nautilus Trader.

## **Quick Start**

### **Installation**

```bash
# Sync dependencies with UV
uv sync --all-groups --all-extras

# Build Nautilus Trader
uv run --no-sync python build.py

# Build Rust components
cargo build --release -p binance_trade_ws

# Install Python bindings
cd crates/adapters/binance_trade_ws
maturin develop --release
```

### **Basic Usage**

```python
from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWSClient
from nautilus_trader.common.component import LiveClock

client = BinanceTradeWSClient(
    clock=LiveClock(),
    api_key="your_api_key",
    ed25519_private_key="your_ed25519_private_key",
    testnet=True
)

# Connect and place order
await client.connect()
await client.authenticate()

response = await client.place_order(
    symbol="BTCUSDT",
    side="BUY",
    order_type="LIMIT",
    quantity="0.001",
    price="50000.00"
)
```

## **Configuration**

```python
from nautilus_trader.adapters.binance.config import BinanceExecClientConfig

config = BinanceExecClientConfig(
    api_key="your_api_key",
    ed25519_private_key="your_ed25519_private_key",
    testnet=True,
    use_trade_websocket=True  # Enable WebSocket trading
)
```

## **Testing**

```bash
# Run unit tests
python -m pytest tests/unit_tests/adapters/binance/websocket/ -v

# Test categories
python -m pytest tests/unit_tests/adapters/binance/websocket/test_connection.py -v
python -m pytest tests/unit_tests/adapters/binance/websocket/test_order_operations.py -v
python -m pytest tests/unit_tests/adapters/binance/websocket/test_error_handling.py -v

# Run latency benchmark (requires environment variables)
export BINANCE_API_KEY="your_api_key"
export BINANCE_ED25519_PRIVATE_KEY="your_ed25519_private_key"
python examples/live/binance/websocket_vs_rest_latency_test.py
```

The Binance Trade WebSocket client provides:

- **Real-time order operations**: place, modify, cancel, status
- **Ed25519 session authentication**: Fast session logon without per-request signing
- **Execution report processing**: Real-time order updates
- **Integration with Nautilus**: Seamless order management workflow

> **Note**: After `session.logon` with Ed25519, all subsequent requests are sent without signatures, providing significant latency improvements over REST API.

## **Architecture**

### **Component Flow**

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Trading       │    │   Execution     │    │   Trade WS      │
│   Strategy      │───▶│   Client WS     │───▶│   Client        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │   PyO3 Bridge   │    │   Rust WS       │
                       │                 │───▶│   Client        │
                       └─────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
                                              ┌─────────────────┐
                                              │   Binance       │
                                              │   Trade API     │
                                              └─────────────────┘
```

### **File Structure & Responsibilities**

```
binance_trade_ws/                             # Rust WebSocket Client Crate
├── Cargo.toml                                # Rust dependencies and metadata
├── src/
│   ├── lib.rs                                # Main library exports and PyO3 module
│   ├── client.rs                             # Core WebSocket client implementation
│   │                                         # - Connection management and reconnection
│   │                                         # - Session authentication with Binance
│   │                                         # - Order operations (place/modify/cancel/status)
│   │                                         # - Message handling and routing
│   │                                         # - Heartbeat and keep-alive management
│   │
│   ├── auth.rs                               # Authentication module
│   │                                         # - API key/Ed25519 private key validation
│   │                                         # - Ed25519 signature generation for session.logon
│   │                                         # - Session authentication management
│   │                                         # - Signature-free requests after authentication
│   │
│   ├── types.rs                              # Type definitions and serialization
│   │                                         # - Order request/response structures
│   │                                         # - ExecutionReport definitions
│   │                                         # - WebSocket message types
│   │                                         # - JSON serialization/deserialization
│   │
│   ├── error.rs                              # Error handling and definitions
│   │                                         # - Custom error types for WebSocket operations
│   │                                         # - API error mapping and conversion
│   │                                         # - Error propagation and logging
│   │
│   └── python/
│       ├── mod.rs                            # Python module exports
│       └── websocket.rs                      # PyO3 bindings and Python interface
│                                             # - Python class wrappers for Rust types
│                                             # - Async method bindings
│                                             # - Type conversion between Python and Rust
│                                             # - Error handling for Python calls
│
nautilus_trader/adapters/binance/
├── execution_ws.py                           # Enhanced execution client with WebSocket support
│                                             # - Extends BinanceCommonExecutionClient
│                                             # - Integrates WebSocket for order operations
│                                             # - Handles execution reports and converts to Nautilus events
│                                             # - Provides fallback to HTTP on WebSocket failures
│
├── websocket/
│   ├── __init__.py                           # Package initialization
│   ├── client.py                             # Public WebSocket client (market data)
│   │                                         # - Handles public market data streams
│   │                                         # - Subscription management
│   │
│   └── trade_client.py                       # Trade WebSocket client wrapper
│                                             # - Python wrapper for Rust WebSocket client
│                                             # - Async interface for order operations
│                                             # - Execution report handler setup
│                                             # - Error handling and logging
│
└── config.py                                # Configuration classes
                                              # - BinanceExecClientConfig with WebSocket options
                                              # - API credentials and connection settings
```

### **Data Flow**

```

1. Authentication:
   trade_client.py → PyO3 → auth.rs → Binance API 


2. Order Placement:
   Strategy → execution_ws.py → trade_client.py → PyO3 → client.rs → Binance API

```

## **Build Instructions**

### **Prerequisites**

- Rust 1.88+ with cargo
- Python 3.11-3.13
- Nautilus Trader development environment