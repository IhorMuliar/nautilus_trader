#!/usr/bin/env python3
"""
Simple test script to verify OKX bar request implementation.
"""

import asyncio
import sys
import os
from datetime import datetime, timedelta, timezone
from unittest.mock import MagicMock, patch

# Add the current directory to the Python path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from nautilus_trader.adapters.okx.data import OKXDataClient
from nautilus_trader.adapters.okx.config import OKXDataClientConfig
from nautilus_trader.adapters.okx.providers import OKXInstrumentProvider
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.core.nautilus_pyo3 import OKXInstrumentType
from nautilus_trader.data.messages import RequestBars
from nautilus_trader.model.data import BarType
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.test_kit.stubs.identifiers import TestIdStubs


async def test_bar_request_implementation():
    """Test the OKX bar request implementation."""
    print("Testing OKX bar request implementation...")
    
    # Create basic configuration
    config = OKXDataClientConfig(
        api_key="test_key",
        api_secret="test_secret",
        api_passphrase="test_passphrase",
        is_demo=True,
        instrument_types=(OKXInstrumentType.SWAP,),
    )
    
    # Create dependencies
    clock = LiveClock()
    msgbus = MessageBus(trader_id=TestIdStubs.trader_id(), clock=clock)
    cache = MagicMock()
    
    # Mock the HTTP client
    http_client = MagicMock(spec=nautilus_pyo3.OKXHttpClient)
    
    # Create a proper instrument provider
    instrument_provider = OKXInstrumentProvider(
        client=http_client,
        clock=clock,
        instrument_types=(OKXInstrumentType.SWAP,),
    )
    
    # Mock the initialize method to avoid actual HTTP requests
    instrument_provider.initialize = MagicMock()
    
    # Create the client
    client = OKXDataClient(
        loop=asyncio.get_event_loop(),
        client=http_client,
        msgbus=msgbus,
        cache=cache,
        clock=clock,
        instrument_provider=instrument_provider,
        config=config,
        name=None,
    )
    
    # Create a bar type
    bar_type = BarType.from_str("BTC-USDT-SWAP.OKX-1-MINUTE-LAST-EXTERNAL")
    
    # Test scenarios
    test_cases = [
        {
            "name": "Recent data (10 days) with reasonable limit",
            "start": datetime.now(timezone.utc) - timedelta(days=10),
            "end": datetime.now(timezone.utc),
            "limit": 200,
            "expected_limit": 200,
        },
        {
            "name": "Recent data (10 days) with excessive limit",
            "start": datetime.now(timezone.utc) - timedelta(days=10),
            "end": datetime.now(timezone.utc),
            "limit": 500,
            "expected_limit": 300,
        },
        {
            "name": "Historical data (150 days) with excessive limit",
            "start": datetime.now(timezone.utc) - timedelta(days=150),
            "end": datetime.now(timezone.utc) - timedelta(days=120),
            "limit": 200,
            "expected_limit": 100,
        },
        {
            "name": "No time range with default limit",
            "start": None,
            "end": None,
            "limit": None,
            "expected_limit": 100,
        },
        {
            "name": "Zero limit should use default",
            "start": None,
            "end": None,
            "limit": 0,
            "expected_limit": 100,
        },
    ]
    
    for test_case in test_cases:
        print(f"\n🧪 Testing: {test_case['name']}")
        
        # Create request
        request = RequestBars(
            client_id=ClientId("OKX"),
            id=TestIdStubs.uuid(),
            bar_type=bar_type,
            start=test_case['start'],
            end=test_case['end'],
            limit=test_case['limit'],
            ts_init=0,
        )
        
        # Mock the HTTP client and handle_bars method
        http_client.request_bars = MagicMock(return_value=[])
        client._handle_bars = MagicMock()
        
        # Execute the request
        await client._request_bars(request)
        
        # Verify the results
        http_client.request_bars.assert_called_once()
        call_kwargs = http_client.request_bars.call_args[1]
        
        # Check parameters
        assert call_kwargs["start"] == test_case['start'], f"Start time mismatch: {call_kwargs['start']} != {test_case['start']}"
        assert call_kwargs["end"] == test_case['end'], f"End time mismatch: {call_kwargs['end']} != {test_case['end']}"
        assert call_kwargs["limit"] == test_case['expected_limit'], f"Limit mismatch: {call_kwargs['limit']} != {test_case['expected_limit']}"
        
        print(f"✅ Passed: limit={call_kwargs['limit']}")
        
        # Reset mock for next test
        http_client.reset_mock()
    
    print("\n🎉 All tests passed!")


if __name__ == "__main__":
    asyncio.run(test_bar_request_implementation())
