"""
Unit tests for Binance Trade WebSocket order operations.
"""

import asyncio
import pytest
from unittest.mock import patch

from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWSClient
from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWebSocketError
from nautilus_trader.common.component import LiveClock


class TestBinanceTradeWebSocketOrderOperations:

    def setup_method(self):
        self.clock = LiveClock()
        self.api_key = "test_api_key"
        self.ed25519_private_key = "test_ed25519_private_key_1234567890123456"
        
        self.client = BinanceTradeWSClient(
            clock=self.clock,
            api_key=self.api_key,
            ed25519_private_key=self.ed25519_private_key,
            testnet=True
        )
        
    @pytest.mark.asyncio
    async def test_place_order_success(self):
        expected_response = {
            "orderId": 12345,
            "symbol": "BTCUSDT",
            "status": "NEW",
            "clientOrderId": "test_order_123",
            "price": "50000.00",
            "origQty": "0.001",
            "executedQty": "0.000",
            "cummulativeQuoteQty": "0.00",
            "timeInForce": "GTC",
            "type": "LIMIT",
            "side": "BUY"
        }
        
        with patch.object(self.client._client, 'place_order') as mock_place:
            mock_place.return_value = asyncio.Future()
            mock_place.return_value.set_result(expected_response)
            
            result = await self.client.place_order(
                symbol="BTCUSDT",
                side="BUY",
                order_type="LIMIT",
                quantity="0.001",
                price="50000.00",
                time_in_force="GTC",
                new_client_order_id="test_order_123"
            )
            assert result["orderId"] == 12345
            assert result["symbol"] == "BTCUSDT"
            assert result["side"] == "BUY"
            assert result["status"] == "NEW"

    @pytest.mark.asyncio
    async def test_cancel_order_success(self):
        expected_response = {
            "orderId": 12345,
            "symbol": "BTCUSDT",
            "status": "CANCELED",
            "clientOrderId": "test_order_123",
            "origClientOrderId": "test_order_123"
        }
        
        with patch.object(self.client._client, 'cancel_order') as mock_cancel:
            mock_cancel.return_value = asyncio.Future()
            mock_cancel.return_value.set_result(expected_response)
            
            result = await self.client.cancel_order(
                symbol="BTCUSDT",
                orig_client_order_id="test_order_123"
            )
            
            assert result["orderId"] == 12345
            assert result["status"] == "CANCELED"
            
    @pytest.mark.asyncio
    async def test_cancel_order_not_found(self):
        with patch.object(self.client._client, 'cancel_order') as mock_cancel:
            mock_cancel.side_effect = Exception("Order not found")
            
            with pytest.raises(BinanceTradeWebSocketError):
                await self.client.cancel_order(
                    symbol="BTCUSDT",
                    orig_client_order_id="non_existent_order"
                )
                
    @pytest.mark.asyncio
    async def test_modify_order_success(self):
        expected_response = {
            "orderId": 12345,
            "symbol": "BTCUSDT",
            "status": "NEW",
            "clientOrderId": "test_order_123",
            "price": "51000.00",
            "origQty": "0.002",
            "type": "LIMIT",
            "side": "BUY"
        }
        
        with patch.object(self.client._client, 'modify_order') as mock_modify:
            mock_modify.return_value = asyncio.Future()
            mock_modify.return_value.set_result(expected_response)
            
            result = await self.client.modify_order(
                symbol="BTCUSDT",
                side="BUY",
                orig_client_order_id="test_order_123",
                quantity="0.002",
                price="51000.00"
            )
            
            assert result["orderId"] == 12345
            assert result["price"] == "51000.00"
            assert result["origQty"] == "0.002"
            
    @pytest.mark.asyncio
    async def test_modify_order_invalid_params(self):
        with patch.object(self.client._client, 'modify_order') as mock_modify:
            mock_modify.side_effect = Exception("Invalid modification parameters")
            
            with pytest.raises(BinanceTradeWebSocketError):
                await self.client.modify_order(
                    symbol="BTCUSDT",
                    side="BUY",
                    orig_client_order_id="test_order_123",
                    quantity="0.0001",  # Too small
                    price="invalid_price"
                )
                
    @pytest.mark.asyncio
    async def test_get_order_status_success(self):
        """Test successful order status query."""
        expected_response = {
            "orderId": 12345,
            "symbol": "BTCUSDT",
            "status": "PARTIALLY_FILLED",
            "clientOrderId": "test_order_123",
            "price": "50000.00",
            "origQty": "0.001",
            "executedQty": "0.0005",
            "cummulativeQuoteQty": "25.00",
            "timeInForce": "GTC",
            "type": "LIMIT",
            "side": "BUY"
        }
        
        with patch.object(self.client._client, 'get_order_status') as mock_status:
            mock_status.return_value = asyncio.Future()
            mock_status.return_value.set_result(expected_response)
            
            result = await self.client.get_order_status(
                symbol="BTCUSDT",
                orig_client_order_id="test_order_123"
            )
            
            assert result["orderId"] == 12345
            assert result["status"] == "PARTIALLY_FILLED"
            assert result["executedQty"] == "0.0005"

    @pytest.mark.asyncio
    async def test_client_initialization_invalid_key_length(self):
        try:
            BinanceTradeWSClient(
                clock=self.clock,
                api_key=self.api_key,
                ed25519_private_key="",
                testnet=True
            )
            assert False, "Expected ValueError to be raised"
        except ValueError as e:
            assert "Ed25519 private key cannot be empty" in str(e)