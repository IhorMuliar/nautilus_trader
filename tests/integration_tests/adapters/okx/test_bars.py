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
import pytest
from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock, patch

from nautilus_trader.adapters.okx.config import OKXDataClientConfig
from nautilus_trader.adapters.okx.data import OKXDataClient
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.data.messages import RequestBars
from nautilus_trader.model.data import BarType
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.test_kit.stubs.identifiers import TestIdStubs


class TestOKXBarRequests:
    """Test cases for OKX bar request functionality."""

    @pytest.fixture
    def config(self):
        return OKXDataClientConfig(
            api_key="test_key",
            api_secret="test_secret",
            api_passphrase="test_passphrase",
            is_demo=True,
        )

    @pytest.fixture
    def client(self, config):
        # Create minimal client setup
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        
        clock = LiveClock()
        msgbus = MessageBus(trader_id=TestIdStubs.trader_id(), clock=clock)
        cache = MagicMock()
        
        # Mock the required parameters
        http_client = MagicMock(spec=nautilus_pyo3.OKXHttpClient)
        instrument_provider = MagicMock()
        
        client = OKXDataClient(
            loop=loop,
            client=http_client,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
            config=config,
            name=None,
        )
        
        return client

    @pytest.mark.asyncio
    async def test_request_bars_recent_data_no_limit_clamping(self, client):
        """Test that recent data (< 100 days) uses regular endpoint with no limit clamping."""
        # Setup
        bar_type = BarType.from_str("BTC-USDT-SWAP.OKX-1-MINUTE-LAST-EXTERNAL")
        now = datetime.now(timezone.utc)
        start = now - timedelta(days=10)  # Recent data
        end = now
        limit = 200  # Within regular endpoint limit
        
        request = RequestBars(
            client_id=ClientId("OKX"),
            id=TestIdStubs.uuid(),
            bar_type=bar_type,
            start=start,
            end=end,
            limit=limit,
            ts_init=0,
        )
        
        # Mock the HTTP client and handler
        mock_bars = []
        client._http_client.request_bars = AsyncMock(return_value=mock_bars)
        client._handle_bars = MagicMock()
        
        # Execute
        await client._request_bars(request)
        
        # Verify
        client._http_client.request_bars.assert_called_once()
        call_kwargs = client._http_client.request_bars.call_args[1]
        assert call_kwargs["start"] == start
        assert call_kwargs["end"] == end
        assert call_kwargs["limit"] == 200  # Should not be clamped

    @pytest.mark.asyncio
    async def test_request_bars_recent_data_with_limit_clamping(self, client):
        """Test that recent data with excessive limit gets clamped to 300."""
        # Setup
        bar_type = BarType.from_str("BTC-USDT-SWAP.OKX-1-MINUTE-LAST-EXTERNAL")
        now = datetime.now(timezone.utc)
        start = now - timedelta(days=10)  # Recent data
        end = now
        limit = 500  # Exceeds regular endpoint limit
        
        request = RequestBars(
            client_id=ClientId("OKX"),
            id=TestIdStubs.uuid(),
            bar_type=bar_type,
            start=start,
            end=end,
            limit=limit,
            ts_init=0,
        )
        
        # Mock the HTTP client and handler
        mock_bars = []
        client._http_client.request_bars = AsyncMock(return_value=mock_bars)
        client._handle_bars = MagicMock()
        
        # Execute
        await client._request_bars(request)
        
        # Verify
        client._http_client.request_bars.assert_called_once()
        call_kwargs = client._http_client.request_bars.call_args[1]
        assert call_kwargs["start"] == start
        assert call_kwargs["end"] == end
        assert call_kwargs["limit"] == 300  # Should be clamped to regular endpoint limit

    @pytest.mark.asyncio
    async def test_request_bars_historical_data_with_limit_clamping(self, client):
        """Test that historical data (> 100 days) gets limit clamped to 100."""
        # Setup
        bar_type = BarType.from_str("BTC-USDT-SWAP.OKX-1-MINUTE-LAST-EXTERNAL")
        now = datetime.now(timezone.utc)
        start = now - timedelta(days=150)  # Historical data
        end = now - timedelta(days=120)
        limit = 200  # Exceeds history endpoint limit
        
        request = RequestBars(
            client_id=ClientId("OKX"),
            id=TestIdStubs.uuid(),
            bar_type=bar_type,
            start=start,
            end=end,
            limit=limit,
            ts_init=0,
        )
        
        # Mock the HTTP client and handler
        mock_bars = []
        client._http_client.request_bars = AsyncMock(return_value=mock_bars)
        client._handle_bars = MagicMock()
        
        # Execute
        await client._request_bars(request)
        
        # Verify
        client._http_client.request_bars.assert_called_once()
        call_kwargs = client._http_client.request_bars.call_args[1]
        assert call_kwargs["start"] == start
        assert call_kwargs["end"] == end
        assert call_kwargs["limit"] == 100  # Should be clamped to history endpoint limit

    @pytest.mark.asyncio
    async def test_request_bars_no_time_range_uses_default_limit(self, client):
        """Test that requests without time range use default limit."""
        # Setup
        bar_type = BarType.from_str("BTC-USDT-SWAP.OKX-1-MINUTE-LAST-EXTERNAL")
        
        request = RequestBars(
            client_id=ClientId("OKX"),
            id=TestIdStubs.uuid(),
            bar_type=bar_type,
            start=None,
            end=None,
            limit=None,
            ts_init=0,
        )
        
        # Mock the HTTP client and handler
        mock_bars = []
        client._http_client.request_bars = AsyncMock(return_value=mock_bars)
        client._handle_bars = MagicMock()
        
        # Execute
        await client._request_bars(request)
        
        # Verify
        client._http_client.request_bars.assert_called_once()
        call_kwargs = client._http_client.request_bars.call_args[1]
        assert call_kwargs["start"] is None
        assert call_kwargs["end"] is None
        assert call_kwargs["limit"] == 100  # Should use default limit

    @pytest.mark.asyncio
    async def test_request_bars_zero_limit_uses_default(self, client):
        """Test that zero limit uses default limit."""
        # Setup
        bar_type = BarType.from_str("BTC-USDT-SWAP.OKX-1-MINUTE-LAST-EXTERNAL")
        
        request = RequestBars(
            client_id=ClientId("OKX"),
            id=TestIdStubs.uuid(),
            bar_type=bar_type,
            start=None,
            end=None,
            limit=0,  # Zero limit
            ts_init=0,
        )
        
        # Mock the HTTP client and handler
        mock_bars = []
        client._http_client.request_bars = AsyncMock(return_value=mock_bars)
        client._handle_bars = MagicMock()
        
        # Execute
        await client._request_bars(request)
        
        # Verify
        client._http_client.request_bars.assert_called_once()
        call_kwargs = client._http_client.request_bars.call_args[1]
        assert call_kwargs["start"] is None
        assert call_kwargs["end"] is None
        assert call_kwargs["limit"] == 100  # Should use default limit
