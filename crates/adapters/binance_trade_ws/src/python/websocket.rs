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

use std::sync::Arc;
use tokio::sync::Mutex;

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3_async_runtimes;

use crate::client::BinanceTradeWebSocketClient as RustClient;
use crate::types::{
    BinanceTradeConfig as RustConfig,
    OrderPlaceParams as RustOrderPlaceParams,
    OrderResponse as RustOrderResponse,
    ExecutionReport as RustExecutionReport,
};

#[pyclass(name = "BinanceTradeConfig")]
#[derive(Clone)]
pub struct BinanceTradeConfig {
    pub inner: RustConfig,
}

#[pymethods]
impl BinanceTradeConfig {
    #[new]
    #[pyo3(signature = (api_key, api_secret, testnet = false, recv_window = None, heartbeat_interval = None))]
    pub fn new(
        api_key: String,
        api_secret: String,
        testnet: bool,
        recv_window: Option<i64>,
        heartbeat_interval: Option<u64>,
    ) -> Self {
        Self {
            inner: RustConfig {
                api_key,
                api_secret,
                testnet,
                recv_window,
                heartbeat_interval,
            },
        }
    }

    pub fn websocket_url(&self) -> String {
        self.inner.websocket_url().to_string()
    }
}

#[pyclass(name = "OrderPlaceParams")]
#[derive(Clone)]
pub struct OrderPlaceParams {
    pub inner: RustOrderPlaceParams,
}

#[pymethods]
impl OrderPlaceParams {
    #[new]
    #[pyo3(signature = (
        symbol,
        side,
        order_type,
        quantity = None,
        price = None,
        time_in_force = None,
        new_client_order_id = None,
        stop_price = None,
        close_position = None,
        activation_price = None,
        callback_rate = None,
        working_type = None,
        price_protect = None,
        reduce_only = None
    ))]
    pub fn new(
        symbol: String,
        side: String,
        order_type: String,
        quantity: Option<String>,
        price: Option<String>,
        time_in_force: Option<String>,
        new_client_order_id: Option<String>,
        stop_price: Option<String>,
        close_position: Option<bool>,
        activation_price: Option<String>,
        callback_rate: Option<String>,
        working_type: Option<String>,
        price_protect: Option<bool>,
        reduce_only: Option<bool>,
    ) -> Self {
        Self {
            inner: RustOrderPlaceParams {
                symbol,
                side,
                order_type,
                quantity,
                price,
                time_in_force,
                new_client_order_id,
                stop_price,
                close_position,
                activation_price,
                callback_rate,
                working_type,
                price_protect,
                reduce_only,
            },
        }
    }
}

#[pyclass(name = "OrderResponse")]
#[derive(Clone)]
pub struct OrderResponse {
    pub inner: RustOrderResponse,
}

#[pymethods]
impl OrderResponse {
    pub fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            
            dict.set_item("orderId", self.inner.order_id)?;
            dict.set_item("symbol", &self.inner.symbol)?;
            dict.set_item("status", &self.inner.status)?;
            dict.set_item("clientOrderId", &self.inner.client_order_id)?;
            dict.set_item("price", &self.inner.price)?;
            dict.set_item("origQty", &self.inner.orig_qty)?;
            dict.set_item("executedQty", &self.inner.executed_qty)?;
            dict.set_item("cummulativeQuoteQty", &self.inner.cum_quote)?;
            dict.set_item("timeInForce", &self.inner.time_in_force)?;
            dict.set_item("type", &self.inner.order_type)?;
            dict.set_item("side", &self.inner.side)?;
            dict.set_item("updateTime", self.inner.update_time)?;
            dict.set_item("avgPrice", &self.inner.avg_price)?;
            dict.set_item("reduceOnly", self.inner.reduce_only)?;
            dict.set_item("positionSide", &self.inner.position_side)?;
            dict.set_item("stopPrice", &self.inner.stop_price)?;
            dict.set_item("closePosition", self.inner.close_position)?;
            dict.set_item("origType", &self.inner.orig_type)?;
            dict.set_item("activatePrice", &self.inner.activate_price)?;
            dict.set_item("priceRate", &self.inner.price_rate)?;
            dict.set_item("workingType", &self.inner.working_type)?;
            
            Ok(dict.into())
        })
    }
}

#[pyclass(name = "ExecutionReport")]
#[derive(Clone)]
pub struct ExecutionReport {
    pub inner: RustExecutionReport,
}

#[pymethods]
impl ExecutionReport {
    pub fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            
            dict.set_item("eventType", &self.inner.event_type)?;
            dict.set_item("eventTime", self.inner.event_time)?;
            dict.set_item("transactionTime", self.inner.transaction_time)?;
            
            dict.set_item("symbol", &self.inner.order.symbol)?;
            dict.set_item("orderId", self.inner.order.order_id)?;
            dict.set_item("clientOrderId", &self.inner.order.client_order_id)?;
            dict.set_item("side", &self.inner.order.side)?;
            dict.set_item("orderType", &self.inner.order.order_type)?;
            dict.set_item("timeInForce", &self.inner.order.time_in_force)?;
            dict.set_item("quantity", &self.inner.order.quantity)?;
            dict.set_item("price", &self.inner.order.price)?;
            dict.set_item("avgPrice", &self.inner.order.avg_price)?;
            dict.set_item("stopPrice", &self.inner.order.stop_price)?;
            dict.set_item("executionType", &self.inner.order.execution_type)?;
            dict.set_item("orderStatus", &self.inner.order.order_status)?;
            dict.set_item("lastExecutedQuantity", &self.inner.order.last_executed_quantity)?;
            dict.set_item("cumulativeFilledQuantity", &self.inner.order.cumulative_filled_quantity)?;
            dict.set_item("lastExecutedPrice", &self.inner.order.last_executed_price)?;
            dict.set_item("commissionAmount", &self.inner.order.commission_amount)?;
            dict.set_item("commissionAsset", &self.inner.order.commission_asset.clone().unwrap_or_default())?;
            dict.set_item("orderTradeTime", self.inner.order.order_trade_time)?;
            dict.set_item("tradeId", self.inner.order.trade_id)?;
            dict.set_item("bidsNotional", &self.inner.order.bids_notional)?;
            
            Ok(dict.into())
        })
    }
}

#[pyclass(name = "BinanceTradeWebSocketClient")]
pub struct BinanceTradeWebSocketClient {
    inner: Arc<Mutex<RustClient>>,
}

#[pymethods]
impl BinanceTradeWebSocketClient {
    #[new]
    pub fn new(config: &BinanceTradeConfig) -> PyResult<Self> {
        let client = RustClient::new(config.inner.clone())
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create client: {}", e)))?;
        
        Ok(Self { inner: Arc::new(Mutex::new(client)) })
    }

    pub fn connect<'a>(&mut self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            client.connect().await
                .map_err(|e| PyRuntimeError::new_err(format!("Connection failed: {}", e)))
        })
    }

    pub fn disconnect<'a>(&mut self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            client.disconnect().await
                .map_err(|e| PyRuntimeError::new_err(format!("Disconnection failed: {}", e)))
        })
    }

    pub fn is_connected<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            let result = client.is_ready().await;
            Ok(result)
        })
    }

    pub fn is_ready<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            let result = client.is_ready().await;
            Ok(result)
        })
    }

    pub fn connection_state<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            let result = if client.is_ready().await {
                "authenticated"
            } else {
                "disconnected"
            };
            Ok(result.to_string())
        })
    }

    pub fn place_order<'a>(&mut self, params: &OrderPlaceParams, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        let params_clone = params.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            let result = client.place_order(params_clone).await
                .map_err(|e| PyRuntimeError::new_err(format!("Order placement failed: {}", e)))?;
            Ok(OrderResponse { inner: result })
        })
    }

    pub fn cancel_order<'a>(&mut self, py: Python<'a>, symbol: String, order_id: Option<i64>, orig_client_order_id: Option<String>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            let cancel_params = crate::types::OrderCancelParams {
                symbol,
                order_id,
                orig_client_order_id,
            };
            let result = client.cancel_order(cancel_params).await
                .map_err(|e| PyRuntimeError::new_err(format!("Order cancellation failed: {}", e)))?;
            Ok(OrderResponse { inner: result })
        })
    }

    pub fn modify_order<'a>(&mut self, py: Python<'a>, symbol: String, side: String, order_id: Option<i64>, orig_client_order_id: Option<String>, quantity: Option<String>, price: Option<String>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            let modify_params = crate::types::OrderModifyParams {
                symbol,
                side,
                order_id,
                orig_client_order_id,
                quantity,
                price,
            };
            let result = client.modify_order(modify_params).await
                .map_err(|e| PyRuntimeError::new_err(format!("Order modification failed: {}", e)))?;
            Ok(OrderResponse { inner: result })
        })
    }

    pub fn get_order_status<'a>(&mut self, py: Python<'a>, symbol: String, order_id: Option<i64>, orig_client_order_id: Option<String>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let client = inner.lock().await;
            let status_params = crate::types::OrderStatusParams {
                symbol,
                order_id,
                orig_client_order_id,
            };
            let result = client.get_order_status(status_params).await
                .map_err(|e| PyRuntimeError::new_err(format!("Order status query failed: {}", e)))?;
            Ok(OrderResponse { inner: result })
        })
    }

    pub fn set_execution_report_handler(&mut self, handler: PyObject) -> PyResult<()> {
        let inner = self.inner.clone();
        
        let closure = move |execution_report: crate::types::ExecutionReport| {
            Python::with_gil(|py| {
                let py_execution_report = ExecutionReport {
                    inner: execution_report,
                };
                
                match py_execution_report.to_dict() {
                    Ok(report_dict) => {
                        if let Err(e) = handler.call1(py, (report_dict,)) {
                            eprintln!("Error calling Python execution report handler: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error converting execution report to dict: {}", e);
                    }
                }
            });
        };
        
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
        
        runtime.block_on(async {
            let client = inner.lock().await;
            client.set_execution_report_handler(closure).await;
        });
        Ok(())
    }
} 