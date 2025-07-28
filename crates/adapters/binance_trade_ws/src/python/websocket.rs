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
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::client::BinanceTradeWebSocketClient as RustClient;
use crate::types::{
    BinanceTradeConfig as RustConfig,
    OrderPlaceParams as RustOrderPlaceParams,
    OrderModifyParams as RustOrderModifyParams,
    OrderCancelParams as RustOrderCancelParams,
    OrderStatusParams as RustOrderStatusParams,
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
    #[pyo3(signature = (api_key, ed25519_private_key, testnet = false, recv_window = None, heartbeat_interval = None))]
    pub fn new(
        api_key: String,
        ed25519_private_key: String,
        testnet: bool,
        recv_window: Option<i64>,
        heartbeat_interval: Option<u64>,
    ) -> Self {
        Self {
            inner: RustConfig::new(api_key, ed25519_private_key, testnet),
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
    #[pyo3(signature = (symbol, side, order_type, quantity = None, price = None, time_in_force = None, new_client_order_id = None, stop_price = None, close_position = None, activation_price = None, callback_rate = None, working_type = None, price_protect = None, reduce_only = None))]
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
    pub fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        let rust_map = self.inner.to_dict();
        
        for (key, value) in rust_map.iter() {
            match value {
                serde_json::Value::String(s) => dict.set_item(key, s)?,
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        dict.set_item(key, i)?;
                    } else if let Some(f) = n.as_f64() {
                        dict.set_item(key, f)?;
                    } else {
                        dict.set_item(key, n.to_string())?;
                    }
                },
                serde_json::Value::Bool(b) => dict.set_item(key, *b)?,
                serde_json::Value::Null => dict.set_item(key, py.None())?,
                _ => dict.set_item(key, value.to_string())?,
            };
        }
        
        Ok(dict.into())
    }

    pub fn get_order_id(&self) -> i64 {
        self.inner.order_id
    }
    
    pub fn get_symbol(&self) -> String {
        self.inner.symbol.clone()
    }
    
    pub fn get_status(&self) -> String {
        self.inner.status.clone()
    }
}

#[pyclass(name = "ExecutionReport")]
#[derive(Clone)]
pub struct ExecutionReport {
    pub inner: RustExecutionReport,
}

#[pymethods]
impl ExecutionReport {
    pub fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        let rust_map = self.inner.to_dict();
        
        for (key, value) in rust_map.iter() {
            match value {
                serde_json::Value::String(s) => dict.set_item(key, s)?,
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        dict.set_item(key, i)?;
                    } else if let Some(f) = n.as_f64() {
                        dict.set_item(key, f)?;
                    } else {
                        dict.set_item(key, n.to_string())?;
                    }
                },
                serde_json::Value::Bool(b) => dict.set_item(key, *b)?,
                serde_json::Value::Null => dict.set_item(key, py.None())?,
                _ => dict.set_item(key, value.to_string())?,
            };
        }
        
        Ok(dict.into())
    }
}

#[pyclass(name = "BinanceTradeWebSocketClient")]
pub struct BinanceTradeWebSocketClient {
    client: Arc<Mutex<RustClient>>,
}

#[pymethods]
impl BinanceTradeWebSocketClient {
    #[new]
    pub fn new(config: BinanceTradeConfig) -> PyResult<Self> {
        let client = RustClient::new(config.inner)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create client: {}", e)))?;
        
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }

    pub fn set_execution_report_handler<'py>(&self, py: Python<'py>, handler: PyObject) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let client_guard = client.lock().await;
            client_guard.set_execution_report_handler(move |report| {
                Python::with_gil(|py| {
                    let report_wrapper = ExecutionReport { inner: report };
                    if let Err(e) = handler.call1(py, (report_wrapper,)) {
                        eprintln!("Error calling execution report handler: {}", e);
                    }
                });
            }).await;
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    pub fn connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let client_guard = client.lock().await;
            client_guard.connect().await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to connect: {}", e)))?;
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    pub fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let client_guard = client.lock().await;
            client_guard.disconnect().await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to disconnect: {}", e)))?;
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    pub fn is_ready<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let client_guard = client.lock().await;
            let ready = client_guard.is_ready().await;
            Ok(ready)
        })
    }

    pub fn place_order<'py>(&self, py: Python<'py>, params: OrderPlaceParams) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let client_guard = client.lock().await;
            let response = client_guard.place_order(params.inner).await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to place order: {}", e)))?;
            
            Python::with_gil(|py| {
                Ok(Py::new(py, OrderResponse { inner: response })?.into_any())
            })
        })
    }

    pub fn cancel_order<'py>(
        &self, 
        py: Python<'py>, 
        symbol: String, 
        orig_client_order_id: Option<String>, 
        order_id: Option<i64>
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let params = RustOrderCancelParams {
                symbol,
                orig_client_order_id,
                order_id,
            };
            
            let client_guard = client.lock().await;
            let response = client_guard.cancel_order(params).await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to cancel order: {}", e)))?;
            
            Python::with_gil(|py| {
                Ok(Py::new(py, OrderResponse { inner: response })?.into_any())
            })
        })
    }

    pub fn modify_order<'py>(
        &self, 
        py: Python<'py>, 
        symbol: String, 
        side: String, 
        orig_client_order_id: Option<String>, 
        order_id: Option<i64>, 
        quantity: Option<String>, 
        price: Option<String>
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let params = RustOrderModifyParams {
                symbol,
                side,
                orig_client_order_id,
                order_id,
                quantity,
                price,
            };
            
            let client_guard = client.lock().await;
            let response = client_guard.modify_order(params).await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to modify order: {}", e)))?;
            
            Python::with_gil(|py| {
                Ok(Py::new(py, OrderResponse { inner: response })?.into_any())
            })
        })
    }

    pub fn get_order_status<'py>(
        &self, 
        py: Python<'py>, 
        symbol: String, 
        orig_client_order_id: Option<String>, 
        order_id: Option<i64>
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let params = RustOrderStatusParams {
                symbol,
                orig_client_order_id,
                order_id,
            };
            
            let client_guard = client.lock().await;
            let response = client_guard.get_order_status(params).await
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to get order status: {}", e)))?;
            
            Python::with_gil(|py| {
                Ok(Py::new(py, OrderResponse { inner: response })?.into_any())
            })
        })
    }
} 