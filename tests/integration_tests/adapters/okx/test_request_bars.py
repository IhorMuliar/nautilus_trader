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

import asyncio
import unittest
from datetime import datetime, timedelta, timezone
from unittest.mock import MagicMock, patch

import pytest

from nautilus_trader.adapters.okx.config import OKXDataClientConfig
from nautilus_trader.adapters.okx.constants import OKX_VENUE
from nautilus_trader.adapters.okx.data import OKXDataClient
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.data.messages import RequestBars
from nautilus_trader.model.data import Bar
from nautilus_trader.model.data import BarType
from nautilus_trader.model.enums import BarAggregation
from nautilus_trader.model.enums import PriceType
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity
from nautilus_trader.test_kit.stubs.identifiers import TestIdStubs
from nautilus_trader.test_kit.stubs.component import TestComponentStubs
from nautilus_trader.test_kit.stubs.identifiers import TestIdStubs


class TestOKXDataClientRequestBars(unittest.TestCase):
    def setUp(self):
        # Common test setup
        self.loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self.loop)
        
        # Create dependencies
        self.clock = LiveClock()
        self.msgbus = MessageBus(
            trader_id=TestIdStubs.trader_id(),
            clock=self.clock,
        )
        
        # Mock the cache to avoid actual dependencies
        self.cache = MagicMock()
        
        # Config
        self.config = OKXDataClientConfig(
            api_key="MOCK",
            api_secret="MOCK",
            api_passphrase="MOCK",
            is_demo=True,
        )
        
        # Bar type for testing
        self.instrument_id = InstrumentId.from_str("BTC-USDT-SWAP.OKX")
        self.bar_type = BarType.from_str("BTC-USDT-SWAP.OKX-1-MINUTE-LAST-EXTERNAL")

    def tearDown(self):
        self.loop.close()

    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._http_client")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._handle_bars")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._log")
    async def test_request_bars_with_recent_data_uses_correct_endpoint(self, mock_log, mock_handle_bars, mock_http_client):
        # Arrange
        # Create client with mocked dependencies
        client = OKXDataClient(
            loop=self.loop,
            client=MagicMock(spec=nautilus_pyo3.OKXHttpClient),
            msgbus=self.msgbus,
            cache=self.cache,
            clock=self.clock,
            instrument_provider=MagicMock(),
            config=self.config,
            name=None,
        )
        
        # Setup request parameters
        now = datetime.now(timezone.utc)
        start = now - timedelta(days=10)  # 10 days ago (recent data)
        end = now
        limit = 200
        
        # Setup mock responses
        mock_bars = [
            # Mocked Bar list
            Bar(
                bar_type=self.bar_type,
                open=Price(50000, precision=2),
                high=Price(51000, precision=2),
                low=Price(49000, precision=2),
                close=Price(50500, precision=2),
                volume=Quantity(10, precision=8),
                ts_init=0,  # Not important for test
                ts_event=0,  # Not important for test
            )
        ]
        
        # Configure mocks
        mock_http_client.request_bars.return_value = mock_bars
        
        # Create request message
        request = RequestBars(
            client_id=ClientId("OKX"),
            id=TestIdStubs.uuid(),
            bar_type=self.bar_type,
            start=start,
            end=end,
            limit=limit,
            ts_init=0,
        )
        
        # Act
        await client._request_bars(request)
        
        # Assert
        # Verify correct parameters passed to request_bars
        mock_http_client.request_bars.assert_called_once()
        call_args = mock_http_client.request_bars.call_args[1]
        assert call_args["bar_type"].to_str() == self.bar_type.to_str()
        assert call_args["start"] == start
        assert call_args["end"] == end
        assert call_args["limit"] == 200  # Should use regular endpoint limit (max 300)
        
        # Verify handle_bars was called with expected parameters
        mock_handle_bars.assert_called_once()
        
        # Verify debug log was called
        mock_log.debug.assert_called_once()

    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._http_client")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._handle_bars")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._log")
    async def test_request_bars_with_historical_data_uses_history_endpoint(self, mock_log, mock_handle_bars, mock_http_client):
        # Arrange
        # Create client with mocked dependencies
        client = OKXDataClient(
            loop=self.loop,
            client=MagicMock(spec=nautilus_pyo3.OKXHttpClient),
            msgbus=self.msgbus,
            cache=self.cache,
            clock=self.clock,
            instrument_provider=MagicMock(),
            config=self.config,
            name=None,
        )
        
        # Setup request parameters
        now = datetime.now(timezone.utc)
        start = now - timedelta(days=150)  # 150 days ago (historical data)
        end = now - timedelta(days=120)  # 120 days ago
        limit = 200
        
        # Setup mock responses
        mock_bars = [
            # Mocked Bar list
            Bar(
                bar_type=self.bar_type,
                open=Price(50000, precision=2),
                high=Price(51000, precision=2),
                low=Price(49000, precision=2),
                close=Price(50500, precision=2),
                volume=Quantity(10, precision=8),
                ts_init=0,  # Not important for test
                ts_event=0,  # Not important for test
            )
        ]
        
        # Configure mocks
        mock_http_client.request_bars.return_value = mock_bars
        
        # Create request message
        request = TestComponentStubs.request_bars(
            bar_type=self.bar_type,
            start=start,
            end=end,
            limit=limit,
        )
        
        # Act
        await client._request_bars(request)
        
        # Assert
        # Verify correct parameters passed to request_bars
        mock_http_client.request_bars.assert_called_once()
        call_args = mock_http_client.request_bars.call_args[1]
        assert call_args["bar_type"].to_str() == self.bar_type.to_str()
        assert call_args["start"] == start
        assert call_args["end"] == end
        assert call_args["limit"] == 100  # Should be clamped to history endpoint limit (max 100)
        
        # Verify handle_bars was called with expected parameters
        mock_handle_bars.assert_called_once()
        
        # Verify warning log was called for limit clamping
        mock_log.warning.assert_called_once()

    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._http_client")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._handle_bars")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._log")
    async def test_request_bars_without_time_range(self, mock_log, mock_handle_bars, mock_http_client):
        # Arrange
        # Create client with mocked dependencies
        client = OKXDataClient(
            loop=self.loop,
            client=MagicMock(spec=nautilus_pyo3.OKXHttpClient),
            msgbus=self.msgbus,
            cache=self.cache,
            clock=self.clock,
            instrument_provider=MagicMock(),
            config=self.config,
            name=None,
        )
        
        # Setup request parameters (no time range)
        limit = 100
        
        # Setup mock responses
        mock_bars = [
            # Mocked Bar list
            Bar(
                bar_type=self.bar_type,
                open=Price(50000, precision=2),
                high=Price(51000, precision=2),
                low=Price(49000, precision=2),
                close=Price(50500, precision=2),
                volume=Quantity(10, precision=8),
                ts_init=0,  # Not important for test
                ts_event=0,  # Not important for test
            )
        ]
        
        # Configure mocks
        mock_http_client.request_bars.return_value = mock_bars
        
        # Create request message (without start/end)
        request = TestComponentStubs.request_bars(
            bar_type=self.bar_type,
            limit=limit,
        )
        
        # Act
        await client._request_bars(request)
        
        # Assert
        # Verify correct parameters passed to request_bars
        mock_http_client.request_bars.assert_called_once()
        call_args = mock_http_client.request_bars.call_args[1]
        assert call_args["bar_type"].to_str() == self.bar_type.to_str()
        assert call_args["start"] is None
        assert call_args["end"] is None
        assert call_args["limit"] == 100  # Default limit
        
        # Verify handle_bars was called with expected parameters
        mock_handle_bars.assert_called_once()
        
        # Verify debug log was called
        mock_log.debug.assert_called_once()

    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._http_client")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._handle_bars")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._log")
    async def test_request_bars_with_excessive_limit(self, mock_log, mock_handle_bars, mock_http_client):
        # Arrange
        # Create client with mocked dependencies
        client = OKXDataClient(
            loop=self.loop,
            client=MagicMock(spec=nautilus_pyo3.OKXHttpClient),
            msgbus=self.msgbus,
            cache=self.cache,
            clock=self.clock,
            instrument_provider=MagicMock(),
            config=self.config,
            name=None,
        )
        
        # Setup request parameters
        now = datetime.now(timezone.utc)
        start = now - timedelta(days=10)  # 10 days ago (recent data)
        end = now
        limit = 500  # Exceeds OKX limit
        
        # Setup mock responses
        mock_bars = [
            # Mocked Bar list
            Bar(
                bar_type=self.bar_type,
                open=Price(50000, precision=2),
                high=Price(51000, precision=2),
                low=Price(49000, precision=2),
                close=Price(50500, precision=2),
                volume=Quantity(10, precision=8),
                ts_init=0,  # Not important for test
                ts_event=0,  # Not important for test
            )
        ]
        
        # Configure mocks
        mock_http_client.request_bars.return_value = mock_bars
        
        # Create request message
        request = TestComponentStubs.request_bars(
            bar_type=self.bar_type,
            start=start,
            end=end,
            limit=limit,
        )
        
        # Act
        await client._request_bars(request)
        
        # Assert
        # Verify correct parameters passed to request_bars
        mock_http_client.request_bars.assert_called_once()
        call_args = mock_http_client.request_bars.call_args[1]
        assert call_args["bar_type"].to_str() == self.bar_type.to_str()
        assert call_args["start"] == start
        assert call_args["end"] == end
        assert call_args["limit"] == 300  # Should be clamped to regular endpoint limit
        
        # Verify handle_bars was called with expected parameters
        mock_handle_bars.assert_called_once()
        
        # Verify warning log was called for limit clamping
        mock_log.warning.assert_called_once()

    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._http_client")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._handle_bars")
    @patch("nautilus_trader.adapters.okx.data.OKXDataClient._log")
    async def test_request_bars_with_zero_limit(self, mock_log, mock_handle_bars, mock_http_client):
        # Arrange
        # Create client with mocked dependencies
        client = OKXDataClient(
            loop=self.loop,
            client=MagicMock(spec=nautilus_pyo3.OKXHttpClient),
            msgbus=self.msgbus,
            cache=self.cache,
            clock=self.clock,
            instrument_provider=MagicMock(),
            config=self.config,
            name=None,
        )
        
        # Setup request parameters
        limit = 0  # Zero limit should use default
        
        # Setup mock responses
        mock_bars = [
            # Mocked Bar list
            Bar(
                bar_type=self.bar_type,
                open=Price(50000, precision=2),
                high=Price(51000, precision=2),
                low=Price(49000, precision=2),
                close=Price(50500, precision=2),
                volume=Quantity(10, precision=8),
                ts_init=0,  # Not important for test
                ts_event=0,  # Not important for test
            )
        ]
        
        # Configure mocks
        mock_http_client.request_bars.return_value = mock_bars
        
        # Create request message
        request = TestComponentStubs.request_bars(
            bar_type=self.bar_type,
            limit=limit,
        )
        
        # Act
        await client._request_bars(request)
        
        # Assert
        # Verify correct parameters passed to request_bars
        mock_http_client.request_bars.assert_called_once()
        call_args = mock_http_client.request_bars.call_args[1]
        assert call_args["bar_type"].to_str() == self.bar_type.to_str()
        assert call_args["start"] is None
        assert call_args["end"] is None
        assert call_args["limit"] == 100  # Should use default limit
        
        # Verify handle_bars was called with expected parameters
        mock_handle_bars.assert_called_once()
        
        # Verify debug log was called
        mock_log.debug.assert_called_once()
