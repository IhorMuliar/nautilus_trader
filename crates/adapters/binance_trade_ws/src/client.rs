use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ahash::AHashMap;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{interval, timeout, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use crate::auth::BinanceAuth;
use crate::error::{BinanceTradeError, BinanceTradeResult, BinanceApiError};
use crate::types::{
    BinanceTradeConfig, BinanceTradeRequest, BinanceTradeResponse, 
    OrderPlaceParams, OrderModifyParams, OrderCancelParams, OrderStatusParams,
    OrderResponse, ExecutionReport
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticated,
    Reconnecting,
    Error,
}

#[derive(Debug, Clone)]
struct PendingRequest {
    id: String,
    method: String,
    timestamp: Instant,
    timeout_duration: Duration,
}

pub struct BinanceTradeWebSocketClient {
    config: BinanceTradeConfig,
    auth: Arc<Mutex<BinanceAuth>>,
    connection_state: Arc<RwLock<ConnectionState>>,
    
    ws_writer: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    
    request_id_counter: AtomicU64,
    pending_requests: Arc<Mutex<AHashMap<String, PendingRequest>>>,
    response_handlers: Arc<Mutex<AHashMap<String, tokio::sync::oneshot::Sender<BinanceTradeResponse>>>>,
    
    execution_report_handler: Arc<Mutex<Option<Box<dyn Fn(ExecutionReport) + Send + Sync>>>>,
    
    shutdown_signal: Arc<AtomicBool>,
    heartbeat_interval: Duration,
    reconnect_delay: Duration,
    max_reconnect_attempts: u32,
    
    connection_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    heartbeat_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    cleanup_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl BinanceTradeWebSocketClient {

    pub fn new(config: BinanceTradeConfig) -> BinanceTradeResult<Self> {
        let auth = BinanceAuth::new(config.api_key.clone(), config.api_secret.clone());
        auth.validate_credentials()?;

        Ok(Self {
            config: config.clone(),
            auth: Arc::new(Mutex::new(auth)),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            ws_writer: Arc::new(Mutex::new(None)),
            request_id_counter: AtomicU64::new(1),
            pending_requests: Arc::new(Mutex::new(AHashMap::new())),
            response_handlers: Arc::new(Mutex::new(AHashMap::new())),
            execution_report_handler: Arc::new(Mutex::new(None)),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            heartbeat_interval: Duration::from_secs(config.heartbeat_interval.unwrap_or(30)),
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 10,
            connection_task: Arc::new(Mutex::new(None)),
            heartbeat_task: Arc::new(Mutex::new(None)),
            cleanup_task: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn set_execution_report_handler<F>(&self, handler: F)
    where
        F: Fn(ExecutionReport) + Send + Sync + 'static,
    {
        let mut handlers = self.execution_report_handler.lock().await;
        *handlers = Some(Box::new(handler));
    }

    pub async fn connection_state(&self) -> ConnectionState {
        *self.connection_state.read().await
    }

    pub async fn is_ready(&self) -> bool {
        matches!(self.connection_state().await, ConnectionState::Authenticated)
    }

    fn generate_request_id(&self) -> String {
        let id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        format!("req_{}", id)
    }

    pub async fn connect(&self) -> BinanceTradeResult<()> {
        info!("Connecting to Binance Trade WebSocket API");
        self.set_connection_state(ConnectionState::Connecting).await;

        self.shutdown_signal.store(false, Ordering::SeqCst);
        
        let connection_task = self.spawn_connection_task().await;
        *self.connection_task.lock().await = Some(connection_task);
        
        let mut attempts = 0;
        while attempts < 30 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let state = self.connection_state().await;
            match state {
                ConnectionState::Authenticated => {
                    info!("Successfully connected and authenticated to Binance Trade WebSocket");
                    self.start_background_tasks().await;
                    return Ok(());
                }
                ConnectionState::Error => {
                    return Err(BinanceTradeError::Connection("Failed to connect".to_string()));
                }
                _ => {
                    attempts += 1;
                }
            }
        }
        
        Err(BinanceTradeError::Timeout("Connection timeout".to_string()))
    }

    pub async fn disconnect(&self) -> BinanceTradeResult<()> {
        info!("Disconnecting from Binance Trade WebSocket");
        
        self.shutdown_signal.store(true, Ordering::SeqCst);
        self.set_connection_state(ConnectionState::Disconnected).await;
        
        if let Some(writer) = self.ws_writer.lock().await.take() {
            let _ = writer.send(Message::Close(None));
        }
        
        self.shutdown_tasks().await;
        info!("Disconnected from Binance Trade WebSocket");
        Ok(())
    }

    async fn set_connection_state(&self, state: ConnectionState) {
        let mut current_state = self.connection_state.write().await;
        if *current_state != state {
            debug!("Connection state changed: {:?} -> {:?}", *current_state, state);
            *current_state = state;
        }
    }

    async fn spawn_connection_task(&self) -> tokio::task::JoinHandle<()> {
        let ws_writer = self.ws_writer.clone();
        let connection_state = self.connection_state.clone();
        let auth = self.auth.clone();
        let pending_requests = self.pending_requests.clone();
        let response_handlers = self.response_handlers.clone();
        let execution_report_handler = self.execution_report_handler.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        let url = self.config.websocket_url().to_string();
        let reconnect_delay = self.reconnect_delay;
        let max_reconnect_attempts = self.max_reconnect_attempts;

        tokio::spawn(async move {
            let mut reconnect_attempts = 0;
            
            while !shutdown_signal.load(Ordering::SeqCst) && reconnect_attempts < max_reconnect_attempts {
                match Self::establish_connection(
                    &url,
                    &ws_writer,
                    &connection_state,
                    &auth,
                    &pending_requests,
                    &response_handlers,
                    &execution_report_handler,
                    &shutdown_signal,
                ).await {
                    Ok(()) => {
                        debug!("Connection task completed normally");
                        break;
                    }
                    Err(e) => {
                        error!("Connection failed: {}", e);
                        reconnect_attempts += 1;
                        
                        if reconnect_attempts < max_reconnect_attempts {
                            warn!("Reconnecting in {:?} (attempt {}/{})", reconnect_delay, reconnect_attempts, max_reconnect_attempts);
                            
                            {
                                let mut state = connection_state.write().await;
                                *state = ConnectionState::Reconnecting;
                            }
                            
                            tokio::time::sleep(reconnect_delay).await;
                        } else {
                            error!("Max reconnection attempts reached");
                            let mut state = connection_state.write().await;
                            *state = ConnectionState::Error;
                            break;
                        }
                    }
                }
            }
            debug!("Connection task exiting");
        })
    }

    async fn establish_connection(
        url: &str,
        ws_writer: &Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
        connection_state: &Arc<RwLock<ConnectionState>>,
        auth: &Arc<Mutex<BinanceAuth>>,
        pending_requests: &Arc<Mutex<AHashMap<String, PendingRequest>>>,
        response_handlers: &Arc<Mutex<AHashMap<String, tokio::sync::oneshot::Sender<BinanceTradeResponse>>>>,
        execution_report_handler: &Arc<Mutex<Option<Box<dyn Fn(ExecutionReport) + Send + Sync>>>>,
        shutdown_signal: &Arc<AtomicBool>,
    ) -> BinanceTradeResult<()> {
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| BinanceTradeError::Connection(format!("Failed to connect: {}", e)))?;
        
        debug!("WebSocket connection established");
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        
        {
            let mut writer = ws_writer.lock().await;
            *writer = Some(tx);
        }
        
        {
            let mut state = connection_state.write().await;
            *state = ConnectionState::Connected;
        }
        
        let writer_task = {
            let shutdown_signal = shutdown_signal.clone();
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    if shutdown_signal.load(Ordering::SeqCst) {
                        break;
                    }
                    
                    if let Err(e) = ws_sender.send(message).await {
                        error!("Failed to send WebSocket message: {}", e);
                        break;
                    }
                }
                debug!("Writer task exiting");
            })
        };
        
        if let Err(e) = Self::authenticate_session(auth, ws_writer).await {
            error!("Authentication failed: {}", e);
            return Err(e);
        }
        
        {
            let mut state = connection_state.write().await;
            *state = ConnectionState::Authenticated;
        }
        
        info!("WebSocket authenticated successfully");
        
        while let Some(message) = ws_receiver.next().await {
            if shutdown_signal.load(Ordering::SeqCst) {
                break;
            }
            
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = Self::handle_message(
                        &text,
                        pending_requests,
                        response_handlers,
                        execution_report_handler,
                    ).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping, sending pong");
                    if let Some(writer) = ws_writer.lock().await.as_ref() {
                        let _ = writer.send(Message::Pong(data));
                    }
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        writer_task.abort();
        {
            let mut writer = ws_writer.lock().await;
            *writer = None;
        }
        
        Ok(())
    }

    async fn authenticate_session(
        auth: &Arc<Mutex<BinanceAuth>>,
        ws_writer: &Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    ) -> BinanceTradeResult<()> {
        let request_id = "session_logon".to_string();
        let timestamp = BinanceAuth::current_timestamp_ms();
        
        let logon_params = {
            let auth_guard = auth.lock().await;
            auth_guard.create_session_logon_params(timestamp)?
        };
        
        let mut params_map = HashMap::new();
        params_map.insert("apiKey".to_string(), Value::String(logon_params.api_key));
        params_map.insert("signature".to_string(), Value::String(logon_params.signature));
        params_map.insert("timestamp".to_string(), Value::Number(logon_params.timestamp.into()));
        
        let request = BinanceTradeRequest {
            id: request_id.clone(),
            method: "session.logon".to_string(),
            params: params_map,
        };
        
        let request_json = serde_json::to_string(&request)
            .map_err(|e| BinanceTradeError::Json(format!("Failed to serialize logon request: {}", e)))?;
        
        debug!("Sending session logon request");
        
        if let Some(writer) = ws_writer.lock().await.as_ref() {
            writer.send(Message::Text(request_json.into()))
                .map_err(|e| BinanceTradeError::Connection(format!("Failed to send logon request: {}", e)))?;
        } else {
            return Err(BinanceTradeError::Connection("No WebSocket writer available".to_string()));
        }
        
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        {
            let mut auth_guard = auth.lock().await;
            auth_guard.set_session_authenticated("authenticated".to_string());
        }
        
        Ok(())
    }

    async fn handle_message(
        text: &str,
        pending_requests: &Arc<Mutex<AHashMap<String, PendingRequest>>>,
        response_handlers: &Arc<Mutex<AHashMap<String, tokio::sync::oneshot::Sender<BinanceTradeResponse>>>>,
        execution_report_handler: &Arc<Mutex<Option<Box<dyn Fn(ExecutionReport) + Send + Sync>>>>,
    ) -> BinanceTradeResult<()> {
        debug!("Received message: {}", text);
        
        if let Ok(response) = serde_json::from_str::<BinanceTradeResponse>(text) {
            Self::handle_response(response, pending_requests, response_handlers).await?;
            return Ok(());
        }
        
        if let Ok(execution_report) = serde_json::from_str::<ExecutionReport>(text) {
            Self::handle_execution_report(execution_report, execution_report_handler).await?;
            return Ok(());
        }
        
        debug!("Unhandled message type: {}", text);
        Ok(())
    }

    async fn handle_response(
        response: BinanceTradeResponse,
        pending_requests: &Arc<Mutex<AHashMap<String, PendingRequest>>>,
        response_handlers: &Arc<Mutex<AHashMap<String, tokio::sync::oneshot::Sender<BinanceTradeResponse>>>>,
    ) -> BinanceTradeResult<()> {
        {
            let mut pending = pending_requests.lock().await;
            pending.remove(&response.id);
        }
        
        {
            let mut handlers = response_handlers.lock().await;
            if let Some(sender) = handlers.remove(&response.id) {
                let _ = sender.send(response);
            }
        }
        
        Ok(())
    }

    async fn handle_execution_report(
        execution_report: ExecutionReport,
        execution_report_handler: &Arc<Mutex<Option<Box<dyn Fn(ExecutionReport) + Send + Sync>>>>,
    ) -> BinanceTradeResult<()> {
        debug!("Received execution report: {:?}", execution_report);
        
        let handler = execution_report_handler.lock().await;
        if let Some(ref handler_fn) = *handler {
            handler_fn(execution_report);
        }
        
        Ok(())
    }

    async fn start_background_tasks(&self) {
        let heartbeat_task = self.spawn_heartbeat_task();
        *self.heartbeat_task.lock().await = Some(heartbeat_task);
        
        let cleanup_task = self.spawn_cleanup_task();
        *self.cleanup_task.lock().await = Some(cleanup_task);
    }

    fn spawn_heartbeat_task(&self) -> tokio::task::JoinHandle<()> {
        let ws_writer = self.ws_writer.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        let heartbeat_interval = self.heartbeat_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(heartbeat_interval);
            
            while !shutdown_signal.load(Ordering::SeqCst) {
                interval.tick().await;
                
                if let Some(writer) = ws_writer.lock().await.as_ref() {
                    let ping_data = b"heartbeat".to_vec();
                    if let Err(e) = writer.send(Message::Ping(ping_data.into())) {
                        error!("Failed to send heartbeat ping: {}", e);
                        break;
                    }
                    debug!("Sent heartbeat ping");
                }
            }
            
            debug!("Heartbeat task exiting");
        })
    }

    fn spawn_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let pending_requests = self.pending_requests.clone();
        let response_handlers = self.response_handlers.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            while !shutdown_signal.load(Ordering::SeqCst) {
                interval.tick().await;
                
                let now = Instant::now();
                let mut expired_ids = Vec::new();
                
                {
                    let pending = pending_requests.lock().await;
                    for (id, request) in pending.iter() {
                        if now.duration_since(request.timestamp) > request.timeout_duration {
                            expired_ids.push(id.clone());
                        }
                    }
                }
                
                if !expired_ids.is_empty() {
                    let mut pending = pending_requests.lock().await;
                    let mut handlers = response_handlers.lock().await;
                    
                    for id in expired_ids {
                        pending.remove(&id);
                        if let Some(sender) = handlers.remove(&id) {
                            let timeout_response = BinanceTradeResponse {
                                id: id.clone(),
                                status: Some(408),
                                result: None,
                                error: Some(BinanceApiError {
                                    code: -1,
                                    msg: "Request timeout".to_string(),
                                }),
                                rate_limits: None,
                            };
                            let _ = sender.send(timeout_response);
                        }
                        warn!("Request {} timed out", id);
                    }
                }
            }
            
            debug!("Cleanup task exiting");
        })
    }

    async fn shutdown_tasks(&self) {
        if let Some(task) = self.connection_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        
        if let Some(task) = self.heartbeat_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        
        if let Some(task) = self.cleanup_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
    }

    async fn send_request(&self, method: &str, params: HashMap<String, Value>) -> BinanceTradeResult<BinanceTradeResponse> {
        if !self.is_ready().await {
            return Err(BinanceTradeError::Connection("Client not ready".to_string()));
        }
        
        let request_id = self.generate_request_id();
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        let signed_params = {
            let auth = self.auth.lock().await;
            auth.sign_params(params)?
        };
        
        let request = BinanceTradeRequest {
            id: request_id.clone(),
            method: method.to_string(),
            params: signed_params,
        };
        
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), PendingRequest {
                id: request_id.clone(),
                method: method.to_string(),
                timestamp: Instant::now(),
                timeout_duration: Duration::from_secs(30),
            });
        }
        
        {
            let mut handlers = self.response_handlers.lock().await;
            handlers.insert(request_id.clone(), tx);
        }
        
        let request_json = serde_json::to_string(&request)?;
        debug!("Sending request: {}", request_json);
        
        if let Some(writer) = self.ws_writer.lock().await.as_ref() {
            writer.send(Message::Text(request_json.into()))
                .map_err(|e| BinanceTradeError::Connection(format!("Failed to send request: {}", e)))?;
        } else {
            return Err(BinanceTradeError::Connection("No WebSocket writer available".to_string()));
        }
        
        match timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => {
                if let Some(error) = response.error {
                    Err(BinanceTradeError::from(error))
                } else {
                    Ok(response)
                }
            }
            Ok(Err(_)) => Err(BinanceTradeError::Internal("Response channel closed".to_string())),
            Err(_) => Err(BinanceTradeError::Timeout("Request timeout".to_string())),
        }
    }

    // ORDER OPERATIONS
    pub async fn place_order(&self, params: OrderPlaceParams) -> BinanceTradeResult<OrderResponse> {
        let mut request_params = HashMap::new();
        
        request_params.insert("symbol".to_string(), Value::String(params.symbol));
        request_params.insert("side".to_string(), Value::String(params.side));
        request_params.insert("type".to_string(), Value::String(params.order_type));
        
        if let Some(quantity) = params.quantity {
            request_params.insert("quantity".to_string(), Value::String(quantity));
        }
        if let Some(price) = params.price {
            request_params.insert("price".to_string(), Value::String(price));
        }
        if let Some(time_in_force) = params.time_in_force {
            request_params.insert("timeInForce".to_string(), Value::String(time_in_force));
        }
        if let Some(client_order_id) = params.new_client_order_id {
            request_params.insert("newClientOrderId".to_string(), Value::String(client_order_id));
        }
        if let Some(stop_price) = params.stop_price {
            request_params.insert("stopPrice".to_string(), Value::String(stop_price));
        }
        if let Some(close_position) = params.close_position {
            request_params.insert("closePosition".to_string(), Value::Bool(close_position));
        }
        if let Some(activation_price) = params.activation_price {
            request_params.insert("activationPrice".to_string(), Value::String(activation_price));
        }
        if let Some(callback_rate) = params.callback_rate {
            request_params.insert("callbackRate".to_string(), Value::String(callback_rate));
        }
        if let Some(working_type) = params.working_type {
            request_params.insert("workingType".to_string(), Value::String(working_type));
        }
        if let Some(price_protect) = params.price_protect {
            request_params.insert("priceProtect".to_string(), Value::Bool(price_protect));
        }
        if let Some(reduce_only) = params.reduce_only {
            request_params.insert("reduceOnly".to_string(), Value::Bool(reduce_only));
        }
        
        let response = self.send_request("order.place", request_params).await?;
        
        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| BinanceTradeError::Json(format!("Failed to parse order response: {}", e)))
        } else {
            Err(BinanceTradeError::Api {
                code: -1,
                message: "No result in response".to_string(),
            })
        }
    }

    pub async fn modify_order(&self, params: OrderModifyParams) -> BinanceTradeResult<OrderResponse> {
        let mut request_params = HashMap::new();
        
        request_params.insert("symbol".to_string(), Value::String(params.symbol));
        request_params.insert("side".to_string(), Value::String(params.side));
        
        if let Some(order_id) = params.order_id {
            request_params.insert("orderId".to_string(), Value::Number(order_id.into()));
        }
        if let Some(orig_client_order_id) = params.orig_client_order_id {
            request_params.insert("origClientOrderId".to_string(), Value::String(orig_client_order_id));
        }
        if let Some(quantity) = params.quantity {
            request_params.insert("quantity".to_string(), Value::String(quantity));
        }
        if let Some(price) = params.price {
            request_params.insert("price".to_string(), Value::String(price));
        }
        
        let response = self.send_request("order.modify", request_params).await?;
        
        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| BinanceTradeError::Json(format!("Failed to parse order response: {}", e)))
        } else {
            Err(BinanceTradeError::Api {
                code: -1,
                message: "No result in response".to_string(),
            })
        }
    }

    pub async fn cancel_order(&self, params: OrderCancelParams) -> BinanceTradeResult<OrderResponse> {
        let mut request_params = HashMap::new();
        
        request_params.insert("symbol".to_string(), Value::String(params.symbol));
        
        if let Some(order_id) = params.order_id {
            request_params.insert("orderId".to_string(), Value::Number(order_id.into()));
        }
        if let Some(orig_client_order_id) = params.orig_client_order_id {
            request_params.insert("origClientOrderId".to_string(), Value::String(orig_client_order_id));
        }
        
        let response = self.send_request("order.cancel", request_params).await?;
        
        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| BinanceTradeError::Json(format!("Failed to parse order response: {}", e)))
        } else {
            Err(BinanceTradeError::Api {
                code: -1,
                message: "No result in response".to_string(),
            })
        }
    }

    pub async fn get_order_status(&self, params: OrderStatusParams) -> BinanceTradeResult<OrderResponse> {
        let mut request_params = HashMap::new();
        
        request_params.insert("symbol".to_string(), Value::String(params.symbol));
        
        if let Some(order_id) = params.order_id {
            request_params.insert("orderId".to_string(), Value::Number(order_id.into()));
        }
        if let Some(orig_client_order_id) = params.orig_client_order_id {
            request_params.insert("origClientOrderId".to_string(), Value::String(orig_client_order_id));
        }
        
        let response = self.send_request("order.status", request_params).await?;
        
        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| BinanceTradeError::Json(format!("Failed to parse order response: {}", e)))
        } else {
            Err(BinanceTradeError::Api {
                code: -1,
                message: "No result in response".to_string(),
            })
        }
    }
}

impl Drop for BinanceTradeWebSocketClient {
    fn drop(&mut self) {
        self.shutdown_signal.store(true, Ordering::SeqCst);
    }
} 