"""
Unit tests for Binance Trade WebSocket connection and authentication.
"""

import asyncio
import pytest
from unittest.mock import patch, MagicMock

from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWSClient
from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWebSocketError
from nautilus_trader.common.component import LiveClock


class TestBinanceTradeWebSocketConnection:

    def setup_method(self):
        self.clock = LiveClock()
        self.api_key = "test_api_key"
        self.api_secret = "test_api_secret"
        
    def test_client_initialization(self):
        client = BinanceTradeWSClient(
            clock=self.clock,
            api_key=self.api_key,
            api_secret=self.api_secret,
            testnet=True
        )
        
        assert client is not None
        assert client._api_key == self.api_key
        assert client._api_secret == self.api_secret
        assert client._testnet is True
        
    def test_client_initialization_invalid_credentials(self):
        with pytest.raises(Exception):
            BinanceTradeWSClient(
                clock=self.clock,
                api_key="",
                api_secret="",
                testnet=True
            )
            
    def test_client_initialization_missing_credentials(self):
        with pytest.raises(Exception):
            BinanceTradeWSClient(
                clock=self.clock,
                api_key=None,
                api_secret=None,
                testnet=True
            )
            
    @pytest.mark.asyncio
    async def test_connection_state_tracking(self):
        client = BinanceTradeWSClient(
            clock=self.clock,
            api_key=self.api_key,
            api_secret=self.api_secret,
            testnet=True
        )
        
        assert await client.is_connected() is False
        
        with patch.object(client._client, 'connect') as mock_connect:
            mock_connect.return_value = asyncio.Future()
            mock_connect.return_value.set_result(None)
            
            await client.connect()
            
            assert await client.is_connected() is True
            
    @pytest.mark.asyncio
    async def test_connection_timeout(self):
        client = BinanceTradeWSClient(
            clock=self.clock,
            api_key=self.api_key,
            api_secret=self.api_secret,
            testnet=True
        )
        
        with patch.object(client._client, 'connect') as mock_connect:
            mock_connect.side_effect = asyncio.TimeoutError("Connection timeout")
            
            with pytest.raises(BinanceTradeWebSocketError):
                await client.connect()
                
    @pytest.mark.asyncio
    async def test_authentication_flow(self):
        client = BinanceTradeWSClient(
            clock=self.clock,
            api_key=self.api_key,
            api_secret=self.api_secret,
            testnet=True
        )
        
        with patch.object(client._client, 'authenticate') as mock_auth:
            mock_auth.return_value = asyncio.Future()
            mock_auth.return_value.set_result({"result": "success"})
            
            result = await client.authenticate()
            
            assert result is not None
            mock_auth.assert_called_once()