# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

"""
Binance Trade WebSocket client for private API operations.

This module provides a WebSocket client specifically designed for Binance's private Trade API
endpoints, supporting Ed25519 authentication, order operations, and real-time execution reports.
"""

from typing import Any, Callable, Optional

from nautilus_trader.common.component import Logger
from nautilus_trader.adapters.binance.common.enums import BinanceAccountType
from nautilus_trader.adapters.binance.common.credentials import get_ed25519_private_key

try:
    from nautilus_trader.core.nautilus_pyo3 import LiveClock
    from nautilus_trader.core.nautilus_pyo3.binance_trade_ws import (
        BinanceTradeConfig,
        BinanceTradeWebSocketClient,
        ExecutionReport,
        OrderPlaceParams,
        OrderResponse,
    )
    BINANCE_TRADE_WS_AVAILABLE = True
except ImportError:
    BINANCE_TRADE_WS_AVAILABLE = False


class BinanceTradeWebSocketError(Exception):
    """
    Represents an error from the Binance Trade WebSocket client.
    """


class BinanceTradeWSClient:
    """
    Provides a Binance Trade WebSocket client for private API operations.

    This client handles:
    - Ed25519 session authentication with Binance WebSocket API
    - Order placement, modification, cancellation, and status requests
    - Real-time execution report events
    - Automatic reconnection with exponential backoff
    - Heartbeat management (ping/pong)

    Parameters
    ----------
    clock : LiveClock
        The clock for the client.
    api_key : str
        The Binance API key.
    ed25519_private_key : bytes
        The Ed25519 private key (32 bytes).
    testnet : bool, default False
        Whether to connect to the testnet.
    recv_window : int, optional
        The receive window for requests (milliseconds).
    heartbeat_interval : int, optional
        The heartbeat interval (seconds).
    execution_report_handler : Callable[[dict], None], optional
        The handler for execution report events.

    Raises
    ------
    ImportError
        If the Binance Trade WebSocket extension is not available.
    BinanceTradeWebSocketError
        If the client configuration is invalid.
    """

    def __init__(
        self,
        clock: LiveClock,
        api_key: str,
        ed25519_private_key: str,
        testnet: bool = False,
        recv_window: Optional[int] = None,
        heartbeat_interval: Optional[int] = None,
        execution_report_handler: Optional[Callable[[dict], None]] = None,
    ) -> None:
        # Validate parameters first (before checking availability)
        if not api_key or not api_key.strip():
            raise ValueError("API key cannot be empty")

        if not ed25519_private_key or not ed25519_private_key.strip():
            raise ValueError("Ed25519 private key cannot be empty")

        if not BINANCE_TRADE_WS_AVAILABLE:
            raise ImportError(
                "Binance Trade WebSocket extension not available. "
                "Please build with the binance-trade-ws feature enabled."
            )

        self._clock = clock
        self._log = Logger(type(self).__name__)
        
        self._api_key = api_key
        self._ed25519_private_key = ed25519_private_key
        self._testnet = testnet
        
        self._config = BinanceTradeConfig(
            api_key=api_key,
            ed25519_private_key=ed25519_private_key,
            testnet=testnet,
            recv_window=recv_window,
            heartbeat_interval=heartbeat_interval,
        )
        
        try:
            self._client = BinanceTradeWebSocketClient(self._config)
        except Exception as e:
            raise BinanceTradeWebSocketError(f"Failed to create client: {e}") from e
        
        if execution_report_handler:
            self.set_execution_report_handler(execution_report_handler)
        
        self._log.info(f"Created Binance Trade WebSocket client")
        self._log.info(f"Authentication: Ed25519")
        self._log.info(f"Testnet: {testnet}")
        self._log.info(f"WebSocket URL: {self._config.websocket_url()}")

    @property
    def api_key(self) -> str:
        return self._api_key

    @property
    def testnet(self) -> bool:
        return self._testnet

    @property
    def websocket_url(self) -> str:
        return self._config.websocket_url()

    async def connect(self) -> None:
        try:
            self._log.info("Connecting to Binance Trade WebSocket API")
            await self._client.connect()
            self._log.info("Successfully connected and authenticated")
        except Exception as e:
            self._log.error(f"Failed to connect: {e}")
            raise BinanceTradeWebSocketError(f"Connection failed: {e}") from e

    async def disconnect(self) -> None:
        try:
            self._log.info("Disconnecting from Binance Trade WebSocket API")
            await self._client.disconnect()
            self._log.info("Successfully disconnected")
        except Exception as e:
            self._log.error(f"Error during disconnect: {e}")
            raise BinanceTradeWebSocketError(f"Disconnect failed: {e}") from e

    async def is_ready(self) -> bool:
        try:
            return await self._client.is_ready()
        except Exception as e:
            self._log.error(f"Error checking ready state: {e}")
            return False

    async def is_connected(self) -> bool:
        try:
            return await self._client.is_connected()
        except Exception as e:
            self._log.error(f"Error checking connection state: {e}")
            return False

    async def connection_state(self) -> str:
        try:
            return await self._client.connection_state()
        except Exception as e:
            self._log.error(f"Error getting connection state: {e}")
            return "error"

    def set_execution_report_handler(self, handler: Callable[[dict], None]) -> None:
        def wrapper(execution_report: ExecutionReport) -> None:
            try:
                report_dict = execution_report.to_dict()
                handler(report_dict)
            except Exception as e:
                self._log.error(f"Error in execution report handler: {e}")

        try:
            self._client.set_execution_report_handler(wrapper)
            self._log.debug("Execution report handler set")
        except Exception as e:
            self._log.error(f"Failed to set execution report handler: {e}")

    async def place_order(
        self,
        symbol: str,
        side: str,
        order_type: str,
        quantity: Optional[str] = None,
        price: Optional[str] = None,
        time_in_force: Optional[str] = None,
        new_client_order_id: Optional[str] = None,
        stop_price: Optional[str] = None,
        close_position: Optional[bool] = None,
        activation_price: Optional[str] = None,
        callback_rate: Optional[str] = None,
        working_type: Optional[str] = None,
        price_protect: Optional[bool] = None,
        reduce_only: Optional[bool] = None,
    ) -> dict[str, Any]:
        try:
            self._log.debug(f"Placing order: {symbol} {side} {order_type}")
            
            params = OrderPlaceParams(
                symbol=symbol,
                side=side,
                order_type=order_type,
                quantity=quantity,
                price=price,
                time_in_force=time_in_force,
                new_client_order_id=new_client_order_id,
                stop_price=stop_price,
                close_position=close_position,
                activation_price=activation_price,
                callback_rate=callback_rate,
                working_type=working_type,
                price_protect=price_protect,
                reduce_only=reduce_only,
            )
            
            response: OrderResponse = await self._client.place_order(params)
            result = response.to_dict()
            
            self._log.info(f"Order placed successfully: {result['orderId']}")
            return result
            
        except Exception as e:
            self._log.error(f"Failed to place order: {e}")
            raise BinanceTradeWebSocketError(f"Order placement failed: {e}") from e

    async def modify_order(
        self,
        symbol: str,
        side: str,
        order_id: Optional[int] = None,
        orig_client_order_id: Optional[str] = None,
        quantity: Optional[str] = None,
        price: Optional[str] = None,
    ) -> dict[str, Any]:
        try:
            self._log.debug(f"Modifying order: {symbol} {order_id or orig_client_order_id}")
            
            response: OrderResponse = await self._client.modify_order(
                symbol=symbol,
                side=side,
                order_id=order_id,
                orig_client_order_id=orig_client_order_id,
                quantity=quantity,
                price=price,
            )
            result = response.to_dict()
            
            self._log.info(f"Order modified successfully: {result['orderId']}")
            return result
            
        except Exception as e:
            self._log.error(f"Failed to modify order: {e}")
            raise BinanceTradeWebSocketError(f"Order modification failed: {e}") from e

    async def cancel_order(
        self,
        symbol: str,
        order_id: Optional[int] = None,
        orig_client_order_id: Optional[str] = None,
    ) -> dict[str, Any]:
        try:
            self._log.debug(f"Cancelling order: {symbol} {order_id or orig_client_order_id}")
            
            response: OrderResponse = await self._client.cancel_order(
                symbol=symbol,
                order_id=order_id,
                orig_client_order_id=orig_client_order_id,
            )
            result = response.to_dict()
            
            self._log.info(f"Order cancelled successfully: {result['orderId']}")
            return result
            
        except Exception as e:
            self._log.error(f"Failed to cancel order: {e}")
            raise BinanceTradeWebSocketError(f"Order cancellation failed: {e}") from e

    async def get_order_status(
        self,
        symbol: str,
        order_id: Optional[int] = None,
        orig_client_order_id: Optional[str] = None,
    ) -> dict[str, Any]:
        try:
            self._log.debug(f"Getting order status: {symbol} {order_id or orig_client_order_id}")
            
            response: OrderResponse = await self._client.get_order_status(
                symbol=symbol,
                order_id=order_id,
                orig_client_order_id=orig_client_order_id,
            )
            result = response.to_dict()
            
            self._log.debug(f"Order status retrieved: {result['orderId']} - {result['status']}")
            return result
            
        except Exception as e:
            self._log.error(f"Failed to get order status: {e}")
            raise BinanceTradeWebSocketError(f"Order status query failed: {e}") from e 