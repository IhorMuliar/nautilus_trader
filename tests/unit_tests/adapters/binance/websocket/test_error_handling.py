"""
Unit tests for Binance Trade WebSocket error handling.
"""

import asyncio
import pytest
from unittest.mock import patch, MagicMock

from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWSClient
from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWebSocketError
from nautilus_trader.common.component import LiveClock


class TestBinanceTradeWebSocketErrorHandling:

    def setup_method(self):
        self.clock = LiveClock()
        self.api_key = "test_api_key"
        self.api_secret = "test_api_secret"
        
        self.client = BinanceTradeWSClient(
            clock=self.clock,
            api_key=self.api_key,
            api_secret=self.api_secret,
            testnet=True
        )
            
    @pytest.mark.asyncio
    async def test_websocket_disconnection_error(self):
        with patch.object(self.client._client, 'place_order') as mock_place:
            mock_place.side_effect = Exception("WebSocket connection closed")
            
            with pytest.raises(BinanceTradeWebSocketError) as exc_info:
                await self.client.place_order(
                    symbol="BTCUSDT",
                    side="BUY",
                    order_type="LIMIT",
                    quantity="0.001",
                    price="50000.00"
                )
            
            assert "WebSocket connection closed" in str(exc_info.value)
            
    @pytest.mark.asyncio
    async def test_order_not_found_error(self):
        with patch.object(self.client._client, 'cancel_order') as mock_cancel:
            mock_cancel.side_effect = Exception("Order not found")
            
            with pytest.raises(BinanceTradeWebSocketError) as exc_info:
                await self.client.cancel_order(
                    symbol="BTCUSDT",
                    orig_client_order_id="non_existent_order"
                )
            
            assert "Order not found" in str(exc_info.value)
            
    @pytest.mark.asyncio
    async def test_invalid_price_error(self):
        with patch.object(self.client._client, 'place_order') as mock_place:
            mock_place.side_effect = Exception("Invalid price")
            
            with pytest.raises(BinanceTradeWebSocketError) as exc_info:
                await self.client.place_order(
                    symbol="BTCUSDT",
                    side="BUY",
                    order_type="LIMIT",
                    quantity="0.001",
                    price="invalid_price"
                )
            
            assert "Invalid price" in str(exc_info.value)
            
    @pytest.mark.asyncio
    async def test_invalid_quantity_error(self):
        with patch.object(self.client._client, 'place_order') as mock_place:
            mock_place.side_effect = Exception("Invalid quantity")
            
            with pytest.raises(BinanceTradeWebSocketError) as exc_info:
                await self.client.place_order(
                    symbol="BTCUSDT",
                    side="BUY",
                    order_type="LIMIT",
                    quantity="0.0000001",
                    price="50000.00"
                )
            assert "Invalid quantity" in str(exc_info.value)