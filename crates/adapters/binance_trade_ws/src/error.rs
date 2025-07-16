use std::fmt;

use serde::{Deserialize, Serialize};

pub type BinanceTradeResult<T> = Result<T, BinanceTradeError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinanceTradeError {
    Connection(String),
    
    Authentication(String),
    
    Api {
        code: i32,
        message: String,
    },
    
    Json(String),
    
    InvalidParam(String),
    
    RateLimit(String),
    
    Timeout(String),
    
    Session(String),
    
    Internal(String),
}

impl fmt::Display for BinanceTradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "Connection error: {}", msg),
            Self::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            Self::Api { code, message } => write!(f, "API error {}: {}", code, message),
            Self::Json(msg) => write!(f, "JSON error: {}", msg),
            Self::InvalidParam(msg) => write!(f, "Invalid parameter: {}", msg),
            Self::RateLimit(msg) => write!(f, "Rate limit error: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout error: {}", msg),
            Self::Session(msg) => write!(f, "Session error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for BinanceTradeError {}

impl From<serde_json::Error> for BinanceTradeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for BinanceTradeError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::Connection(err.to_string())
    }
}

impl From<anyhow::Error> for BinanceTradeError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceApiError {
    pub code: i32,
    pub msg: String,
}

impl From<BinanceApiError> for BinanceTradeError {
    fn from(err: BinanceApiError) -> Self {
        Self::Api {
            code: err.code,
            message: err.msg,
        }
    }
} 