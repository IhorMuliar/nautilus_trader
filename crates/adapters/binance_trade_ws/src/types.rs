use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::BinanceApiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceTradeConfig {
    pub api_key: String,
    pub ed25519_private_key: String,
    pub testnet: bool,
    pub recv_window: Option<i64>,
    pub heartbeat_interval: Option<u64>,
}

impl BinanceTradeConfig {
    pub fn new(api_key: String, ed25519_private_key: String, testnet: bool) -> Self {
        Self {
            api_key,
            ed25519_private_key,
            testnet,
            recv_window: Some(5000),
            heartbeat_interval: Some(30),
        }
    }

    pub fn websocket_url(&self) -> &str {
        if self.testnet {
            "wss://stream.binancefuture.com/ws-fapi/v1"
        } else {
            "wss://ws-fapi.binance.com/ws-fapi/v1"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceTradeRequest {
    pub id: String,
    pub method: String,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceTradeResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BinanceApiError>,
    #[serde(rename = "rateLimits", skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<Vec<RateLimit>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    #[serde(rename = "rateLimitType")]
    pub rate_limit_type: String,
    pub interval: String,
    #[serde(rename = "intervalNum")]
    pub interval_num: i32,
    pub limit: i32,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogonParams {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub signature: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogonResult {
    #[serde(rename = "listenKey")]
    pub listen_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlaceParams {
    pub symbol: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(rename = "timeInForce", skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<String>,  // GTC, IOC, FOK
    #[serde(rename = "newClientOrderId", skip_serializing_if = "Option::is_none")]
    pub new_client_order_id: Option<String>,
    #[serde(rename = "stopPrice", skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<String>,
    #[serde(rename = "closePosition", skip_serializing_if = "Option::is_none")]
    pub close_position: Option<bool>,
    #[serde(rename = "activationPrice", skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<String>,
    #[serde(rename = "callbackRate", skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<String>,
    #[serde(rename = "workingType", skip_serializing_if = "Option::is_none")]
    pub working_type: Option<String>,
    #[serde(rename = "priceProtect", skip_serializing_if = "Option::is_none")]
    pub price_protect: Option<bool>,
    #[serde(rename = "reduceOnly", skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderModifyParams {
    #[serde(rename = "orderId", skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(rename = "origClientOrderId", skip_serializing_if = "Option::is_none")]
    pub orig_client_order_id: Option<String>,
    pub symbol: String,
    pub side: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelParams {
    pub symbol: String,
    #[serde(rename = "orderId", skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(rename = "origClientOrderId", skip_serializing_if = "Option::is_none")]
    pub orig_client_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatusParams {
    pub symbol: String,
    #[serde(rename = "orderId", skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(rename = "origClientOrderId", skip_serializing_if = "Option::is_none")]
    pub orig_client_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    #[serde(rename = "clientOrderId")]
    pub client_order_id: String,
    #[serde(rename = "cumQty")]
    pub cum_qty: String,
    #[serde(rename = "cumQuote")]
    pub cum_quote: String,
    #[serde(rename = "executedQty")]
    pub executed_qty: String,
    #[serde(rename = "orderId")]
    pub order_id: i64,
    #[serde(rename = "avgPrice")]
    pub avg_price: String,
    #[serde(rename = "origQty")]
    pub orig_qty: String,
    pub price: String,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: bool,
    pub side: String,
    #[serde(rename = "positionSide")]
    pub position_side: String,
    pub status: String,
    #[serde(rename = "stopPrice")]
    pub stop_price: String,
    #[serde(rename = "closePosition")]
    pub close_position: bool,
    pub symbol: String,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    #[serde(rename = "origType")]
    pub orig_type: String,
    #[serde(rename = "activatePrice")]
    pub activate_price: String,
    #[serde(rename = "priceRate")]
    pub price_rate: String,
    #[serde(rename = "updateTime")]
    pub update_time: i64,
    #[serde(rename = "workingType")]
    pub working_type: String,
    #[serde(rename = "priceProtect")]
    pub price_protect: bool,
}

impl OrderResponse {
    pub fn to_dict(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let mut map = std::collections::HashMap::new();
        map.insert("clientOrderId".to_string(), serde_json::Value::String(self.client_order_id.clone()));
        map.insert("cumQty".to_string(), serde_json::Value::String(self.cum_qty.clone()));
        map.insert("cumQuote".to_string(), serde_json::Value::String(self.cum_quote.clone()));
        map.insert("executedQty".to_string(), serde_json::Value::String(self.executed_qty.clone()));
        map.insert("orderId".to_string(), serde_json::Value::Number(self.order_id.into()));
        map.insert("avgPrice".to_string(), serde_json::Value::String(self.avg_price.clone()));
        map.insert("origQty".to_string(), serde_json::Value::String(self.orig_qty.clone()));
        map.insert("price".to_string(), serde_json::Value::String(self.price.clone()));
        map.insert("reduceOnly".to_string(), serde_json::Value::Bool(self.reduce_only));
        map.insert("side".to_string(), serde_json::Value::String(self.side.clone()));
        map.insert("positionSide".to_string(), serde_json::Value::String(self.position_side.clone()));
        map.insert("status".to_string(), serde_json::Value::String(self.status.clone()));
        map.insert("stopPrice".to_string(), serde_json::Value::String(self.stop_price.clone()));
        map.insert("closePosition".to_string(), serde_json::Value::Bool(self.close_position));
        map.insert("symbol".to_string(), serde_json::Value::String(self.symbol.clone()));
        map.insert("timeInForce".to_string(), serde_json::Value::String(self.time_in_force.clone()));
        map.insert("type".to_string(), serde_json::Value::String(self.order_type.clone()));
        map.insert("origType".to_string(), serde_json::Value::String(self.orig_type.clone()));
        map.insert("activatePrice".to_string(), serde_json::Value::String(self.activate_price.clone()));
        map.insert("priceRate".to_string(), serde_json::Value::String(self.price_rate.clone()));
        map.insert("updateTime".to_string(), serde_json::Value::Number(self.update_time.into()));
        map.insert("workingType".to_string(), serde_json::Value::String(self.working_type.clone()));
        map.insert("priceProtect".to_string(), serde_json::Value::Bool(self.price_protect));
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "T")]
    pub transaction_time: i64,
    #[serde(rename = "o")]
    pub order: OrderExecutionData,
}

impl ExecutionReport {
    pub fn to_dict(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let mut map = std::collections::HashMap::new();
        map.insert("eventType".to_string(), serde_json::Value::String(self.event_type.clone()));
        map.insert("eventTime".to_string(), serde_json::Value::Number(self.event_time.into()));
        map.insert("transactionTime".to_string(), serde_json::Value::Number(self.transaction_time.into()));
        
        let order_dict = self.order.to_dict();
        map.insert("order".to_string(), serde_json::Value::Object(
            order_dict.into_iter().map(|(k, v)| (k, v)).collect()
        ));
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderExecutionData {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "f")]
    pub time_in_force: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "ap")]
    pub avg_price: String,
    #[serde(rename = "sp")]
    pub stop_price: String,
    #[serde(rename = "x")]
    pub execution_type: String,
    #[serde(rename = "X")]
    pub order_status: String,
    #[serde(rename = "i")]
    pub order_id: i64,
    #[serde(rename = "l")]
    pub last_executed_quantity: String,
    #[serde(rename = "z")]
    pub cumulative_filled_quantity: String,
    #[serde(rename = "L")]
    pub last_executed_price: String,
    #[serde(rename = "n")]
    pub commission_amount: String,
    #[serde(rename = "N")]
    pub commission_asset: Option<String>,
    #[serde(rename = "T")]
    pub order_trade_time: i64,
    #[serde(rename = "t")]
    pub trade_id: i64,
    #[serde(rename = "b")]
    pub bids_notional: String,
    #[serde(rename = "a")]
    pub ask_notional: String,
    #[serde(rename = "m")]
    pub is_maker_side: bool,
    #[serde(rename = "R")]
    pub reduce_only: bool,
    #[serde(rename = "wt")]
    pub working_type: String,
    #[serde(rename = "ot")]
    pub original_order_type: String,
    #[serde(rename = "ps")]
    pub position_side: String,
    #[serde(rename = "cp")]
    pub close_position: bool,
    #[serde(rename = "AP")]
    pub activation_price: String,
    #[serde(rename = "cr")]
    pub callback_rate: String,
    #[serde(rename = "pP")]
    pub price_protect: bool,
    #[serde(rename = "rp")]
    pub realized_profit: String,
    #[serde(rename = "V")]
    pub stop_price_working_type: String,
    #[serde(rename = "pm")]
    pub price_match: String,
    #[serde(rename = "gtd")]
    pub good_till_date: i64,
}

impl OrderExecutionData {
    pub fn to_dict(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let mut map = std::collections::HashMap::new();
        map.insert("symbol".to_string(), serde_json::Value::String(self.symbol.clone()));
        map.insert("clientOrderId".to_string(), serde_json::Value::String(self.client_order_id.clone()));
        map.insert("side".to_string(), serde_json::Value::String(self.side.clone()));
        map.insert("orderType".to_string(), serde_json::Value::String(self.order_type.clone()));
        map.insert("timeInForce".to_string(), serde_json::Value::String(self.time_in_force.clone()));
        map.insert("quantity".to_string(), serde_json::Value::String(self.quantity.clone()));
        map.insert("price".to_string(), serde_json::Value::String(self.price.clone()));
        map.insert("avgPrice".to_string(), serde_json::Value::String(self.avg_price.clone()));
        map.insert("stopPrice".to_string(), serde_json::Value::String(self.stop_price.clone()));
        map.insert("executionType".to_string(), serde_json::Value::String(self.execution_type.clone()));
        map.insert("orderStatus".to_string(), serde_json::Value::String(self.order_status.clone()));
        map.insert("orderId".to_string(), serde_json::Value::Number(self.order_id.into()));
        map.insert("lastExecutedQuantity".to_string(), serde_json::Value::String(self.last_executed_quantity.clone()));
        map.insert("cumulativeFilledQuantity".to_string(), serde_json::Value::String(self.cumulative_filled_quantity.clone()));
        map.insert("lastExecutedPrice".to_string(), serde_json::Value::String(self.last_executed_price.clone()));
        map.insert("commissionAmount".to_string(), serde_json::Value::String(self.commission_amount.clone()));
        if let Some(asset) = &self.commission_asset {
            map.insert("commissionAsset".to_string(), serde_json::Value::String(asset.clone()));
        }
        map.insert("orderTradeTime".to_string(), serde_json::Value::Number(self.order_trade_time.into()));
        map.insert("tradeId".to_string(), serde_json::Value::Number(self.trade_id.into()));
        map.insert("bidsNotional".to_string(), serde_json::Value::String(self.bids_notional.clone()));
        map.insert("askNotional".to_string(), serde_json::Value::String(self.ask_notional.clone()));
        map.insert("isMakerSide".to_string(), serde_json::Value::Bool(self.is_maker_side));
        map.insert("reduceOnly".to_string(), serde_json::Value::Bool(self.reduce_only));
        map.insert("workingType".to_string(), serde_json::Value::String(self.working_type.clone()));
        map.insert("originalOrderType".to_string(), serde_json::Value::String(self.original_order_type.clone()));
        map.insert("positionSide".to_string(), serde_json::Value::String(self.position_side.clone()));
        map.insert("closePosition".to_string(), serde_json::Value::Bool(self.close_position));
        map.insert("activationPrice".to_string(), serde_json::Value::String(self.activation_price.clone()));
        map.insert("callbackRate".to_string(), serde_json::Value::String(self.callback_rate.clone()));
        map.insert("priceProtect".to_string(), serde_json::Value::Bool(self.price_protect));
        map.insert("realizedProfit".to_string(), serde_json::Value::String(self.realized_profit.clone()));
        map.insert("stopPriceWorkingType".to_string(), serde_json::Value::String(self.stop_price_working_type.clone()));
        map.insert("priceMatch".to_string(), serde_json::Value::String(self.price_match.clone()));
        map.insert("goodTillDate".to_string(), serde_json::Value::Number(self.good_till_date.into()));
        map
    }
} 