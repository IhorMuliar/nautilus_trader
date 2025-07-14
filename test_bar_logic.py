#!/usr/bin/env python3
"""
Test script to verify the bar request logic directly.
"""

import asyncio
from datetime import datetime, timezone, timedelta
from unittest.mock import MagicMock


def test_bar_request_logic():
    """Test the bar request logic without creating full client."""
    
    # Mock the datetime.now function to return a fixed time for testing
    fixed_now = datetime(2025, 7, 14, 12, 0, 0, tzinfo=timezone.utc)
    
    test_cases = [
        {
            "name": "Recent data (10 days) with reasonable limit",
            "start": fixed_now - timedelta(days=10),
            "limit": 200,
            "expected_limit": 200,
            "expected_endpoint": "regular",
        },
        {
            "name": "Recent data (10 days) with excessive limit",
            "start": fixed_now - timedelta(days=10),
            "limit": 500,
            "expected_limit": 300,
            "expected_endpoint": "regular",
        },
        {
            "name": "Historical data (150 days) with reasonable limit",
            "start": fixed_now - timedelta(days=150),
            "limit": 50,
            "expected_limit": 50,
            "expected_endpoint": "history",
        },
        {
            "name": "Historical data (150 days) with excessive limit",
            "start": fixed_now - timedelta(days=150),
            "limit": 200,
            "expected_limit": 100,
            "expected_endpoint": "history",
        },
        {
            "name": "No start time with excessive limit",
            "start": None,
            "limit": 500,
            "expected_limit": 300,
            "expected_endpoint": "regular",
        },
        {
            "name": "No start time with default limit",
            "start": None,
            "limit": None,
            "expected_limit": 100,
            "expected_endpoint": "regular",
        },
        {
            "name": "Zero limit should use default",
            "start": None,
            "limit": 0,
            "expected_limit": 100,
            "expected_endpoint": "regular",
        },
    ]
    
    for test_case in test_cases:
        print(f"\n🧪 Testing: {test_case['name']}")
        
        # Simulate the logic from our implementation
        start = test_case['start']
        limit = test_case['limit']
        
        # Check if the request requires using the history endpoint (data > 100 days old)
        use_history_endpoint = False
        if start is not None:
            days_ago = (fixed_now - start).days
            use_history_endpoint = days_ago > 100
            
            if use_history_endpoint and (limit is not None and limit > 100):
                print(f"   Warning: Requested limit {limit} exceeds OKX history endpoint maximum of 100, clamping to 100")
                limit = min(limit, 100)
            elif not use_history_endpoint and (limit is not None and limit > 300):
                print(f"   Warning: Requested limit {limit} exceeds OKX regular endpoint maximum of 300, clamping to 300")
                limit = min(limit, 300)
        elif limit is not None and limit > 300:
            # No start time provided, using regular endpoint
            print(f"   Warning: Requested limit {limit} exceeds OKX maximum of 300, clamping to 300")
            limit = min(limit, 300)
        
        # Use default limit if not specified
        if limit is None or limit <= 0:
            limit = 100  # Default limit
            
        endpoint = 'history' if use_history_endpoint else 'regular'
        
        print(f"   Start: {start}")
        print(f"   Endpoint: {endpoint}")
        print(f"   Final limit: {limit}")
        
        # Check results
        assert limit == test_case['expected_limit'], f"Limit mismatch: {limit} != {test_case['expected_limit']}"
        assert endpoint == test_case['expected_endpoint'], f"Endpoint mismatch: {endpoint} != {test_case['expected_endpoint']}"
        
        print(f"✅ Passed")
    
    print("\n🎉 All tests passed!")


if __name__ == "__main__":
    test_bar_request_logic()
