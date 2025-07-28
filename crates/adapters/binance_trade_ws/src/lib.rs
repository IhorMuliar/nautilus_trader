pub mod auth;
pub mod client;
pub mod error;
pub mod types;

#[cfg(feature = "python")]
pub mod python;

pub use client::BinanceTradeWebSocketClient;
pub use error::{BinanceTradeError, BinanceTradeResult};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BinanceTradeConfig, OrderPlaceParams};

    #[test]
    fn test_config_creation() {
        let ed25519_key = "test_ed25519_private_key_1234567890123456".to_string();
        let config = BinanceTradeConfig::new(
            "test_api_key".to_string(),
            ed25519_key,
            true,
        );

        assert_eq!(config.api_key, "test_api_key");
        assert!(!config.ed25519_private_key.is_empty());
        assert!(config.testnet);
        assert_eq!(config.recv_window, Some(5000));
        assert_eq!(config.heartbeat_interval, Some(30));
        assert_eq!(config.websocket_url(), "wss://stream.binancefuture.com/ws-fapi/v1");
    }

    #[test]
    fn test_config_production_url() {
        let ed25519_key = "test_ed25519_private_key_1234567890123456".to_string();
        let config = BinanceTradeConfig::new(
            "test_api_key".to_string(),
            ed25519_key,
            false,
        );

        assert_eq!(config.websocket_url(), "wss://ws-fapi.binance.com/ws-fapi/v1");
    }

    #[test]
    fn test_order_place_params_creation() {
        let params = OrderPlaceParams {
            symbol: "ETHUSDT".to_string(),
            side: "BUY".to_string(),
            order_type: "LIMIT".to_string(),
            quantity: Some("1.0".to_string()),
            price: Some("2000.0".to_string()),
            time_in_force: Some("GTC".to_string()),
            new_client_order_id: Some("test_order_id".to_string()),
            stop_price: None,
            close_position: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            price_protect: None,
            reduce_only: Some(false),
        };

        assert_eq!(params.symbol, "ETHUSDT");
        assert_eq!(params.side, "BUY");
        assert_eq!(params.order_type, "LIMIT");
        assert_eq!(params.quantity, Some("1.0".to_string()));
        assert_eq!(params.price, Some("2000.0".to_string()));
        assert_eq!(params.new_client_order_id, Some("test_order_id".to_string()));
        assert_eq!(params.reduce_only, Some(false));
    }

    #[test]
    fn test_client_creation() {
        let ed25519_key = "test_ed25519_private_key_1234567890123456".to_string();
        let config = BinanceTradeConfig::new(
            "valid_api_key_123456".to_string(),
            ed25519_key,
            true,
        );

        let result = BinanceTradeWebSocketClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_creation_invalid_credentials() {
        let ed25519_key = "".to_string();
        let config = BinanceTradeConfig::new(
            "short".to_string(),
            ed25519_key,
            true,
        );

        let result = BinanceTradeWebSocketClient::new(config);
        assert!(result.is_err());
    }
} 