use std::time::{SystemTime, UNIX_EPOCH};

use nautilus_cryptography::signing::hmac_signature;
use serde_json::Value;

use crate::error::{BinanceTradeError, BinanceTradeResult};
use crate::types::SessionLogonParams;

#[derive(Debug, Clone)]
pub struct BinanceAuth {
    api_key: String,
    api_secret: String,
    session_authenticated: bool,
    listen_key: Option<String>,
}

impl BinanceAuth {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            session_authenticated: false,
            listen_key: None,
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn is_session_authenticated(&self) -> bool {
        self.session_authenticated
    }

    pub fn listen_key(&self) -> Option<&str> {
        self.listen_key.as_deref()
    }

    pub fn set_session_authenticated(&mut self, listen_key: String) {
        self.session_authenticated = true;
        self.listen_key = Some(listen_key);
    }

    pub fn reset_session(&mut self) {
        self.session_authenticated = false;
        self.listen_key = None;
    }

    pub fn current_timestamp_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as i64
    }

    pub fn create_signature(&self, payload: &str) -> BinanceTradeResult<String> {
        let signature = hmac_signature(&self.api_secret, payload)
            .map_err(|e| BinanceTradeError::Authentication(format!("Failed to create signature: {}", e)))?;
        Ok(signature)
    }

    pub fn create_session_logon_params(&self, timestamp: i64) -> BinanceTradeResult<SessionLogonParams> {
        let payload = format!("timestamp={}", timestamp);
        let signature = self.create_signature(&payload)?;
        
        Ok(SessionLogonParams {
            api_key: self.api_key.clone(),
            signature,
            timestamp,
        })
    }

    pub fn sign_params(&self, mut params: std::collections::HashMap<String, Value>) -> BinanceTradeResult<std::collections::HashMap<String, Value>> {
        if self.session_authenticated {
            return Ok(params);
        }

        let timestamp = Self::current_timestamp_ms();
        params.insert("timestamp".to_string(), Value::Number(timestamp.into()));

        let mut query_params: Vec<String> = Vec::new();
        
        let mut sorted_keys: Vec<&String> = params.keys().collect();
        sorted_keys.sort();
        
        for key in sorted_keys {
            if let Some(value) = params.get(key) {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => serde_json::to_string(value).map_err(|e| {
                        BinanceTradeError::Json(format!("Failed to serialize parameter: {}", e))
                    })?,
                };
                query_params.push(format!("{}={}", key, value_str));
            }
        }
        
        let query_string = query_params.join("&");
        let signature = self.create_signature(&query_string)?;
        
        params.insert("apiKey".to_string(), Value::String(self.api_key.clone()));
        params.insert("signature".to_string(), Value::String(signature));
        
        Ok(params)
    }

    pub fn validate_credentials(&self) -> BinanceTradeResult<()> {
        if self.api_key.is_empty() {
            return Err(BinanceTradeError::Authentication(
                "API key cannot be empty".to_string(),
            ));
        }

        if self.api_secret.is_empty() {
            return Err(BinanceTradeError::Authentication(
                "API secret cannot be empty".to_string(),
            ));
        }

        if self.api_key.len() < 10 {
            return Err(BinanceTradeError::Authentication(
                "API key appears to be too short".to_string(),
            ));
        }

        if self.api_secret.len() < 10 {
            return Err(BinanceTradeError::Authentication(
                "API secret appears to be too short".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_auth_creation() {
        let auth = BinanceAuth::new("test_api_key".to_string(), "test_secret".to_string());
        assert_eq!(auth.api_key(), "test_api_key");
        assert!(!auth.is_session_authenticated());
        assert!(auth.listen_key().is_none());
    }

    #[test]
    fn test_session_management() {
        let mut auth = BinanceAuth::new("test_api_key".to_string(), "test_secret".to_string());
        
        assert!(!auth.is_session_authenticated());
        
        auth.set_session_authenticated("test_listen_key".to_string());
        assert!(auth.is_session_authenticated());
        assert_eq!(auth.listen_key(), Some("test_listen_key"));
        
        auth.reset_session();
        assert!(!auth.is_session_authenticated());
        assert!(auth.listen_key().is_none());
    }

    #[test]
    fn test_timestamp_generation() {
        let ts1 = BinanceAuth::current_timestamp_ms();
        let ts2 = BinanceAuth::current_timestamp_ms();
        
        assert!(ts2 >= ts1);
        assert!(ts2 - ts1 < 1000);
    }

    #[test]
    fn test_credential_validation() {
        let auth = BinanceAuth::new("valid_api_key".to_string(), "valid_secret".to_string());
        assert!(auth.validate_credentials().is_ok());
        
        let auth = BinanceAuth::new("".to_string(), "valid_secret".to_string());
        assert!(auth.validate_credentials().is_err());
        
        let auth = BinanceAuth::new("valid_api_key".to_string(), "".to_string());
        assert!(auth.validate_credentials().is_err());
        
        let auth = BinanceAuth::new("short".to_string(), "alsoshort".to_string());
        assert!(auth.validate_credentials().is_err());
    }

    #[test]
    fn test_session_logon_params() {
        let auth = BinanceAuth::new("test_api_key".to_string(), "test_secret".to_string());
        let timestamp = 1640995200000;
        
        let params = auth.create_session_logon_params(timestamp);
        assert!(params.is_ok());
        
        let params = params.unwrap();
        assert_eq!(params.api_key, "test_api_key");
        assert_eq!(params.timestamp, timestamp);
        assert!(!params.signature.is_empty());
    }
} 