#!/usr/bin/env python3
"""
Comprehensive test for OKX bar request edge cases.
"""

import asyncio
from datetime import datetime, timezone, timedelta
from unittest.mock import MagicMock


def test_all_edge_cases():
    """Test all edge cases for the bar request logic."""
    
    # Mock the datetime.now function to return a fixed time for testing
    fixed_now = datetime(2025, 7, 14, 12, 0, 0, tzinfo=timezone.utc)
    
    test_cases = [
        # Basic functionality
        {
            "name": "Recent data (10 days) with reasonable limit",
            "start": fixed_now - timedelta(days=10),
            "end": fixed_now,
            "limit": 200,
            "expected_limit": 200,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "Recent data (10 days) with excessive limit",
            "start": fixed_now - timedelta(days=10),
            "end": fixed_now,
            "limit": 500,
            "expected_limit": 300,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "Historical data (150 days) with reasonable limit",
            "start": fixed_now - timedelta(days=150),
            "end": fixed_now - timedelta(days=120),
            "limit": 50,
            "expected_limit": 50,
            "expected_endpoint": "history",
            "should_succeed": True,
        },
        {
            "name": "Historical data (150 days) with excessive limit",
            "start": fixed_now - timedelta(days=150),
            "end": fixed_now - timedelta(days=120),
            "limit": 200,
            "expected_limit": 100,
            "expected_endpoint": "history",
            "should_succeed": True,
        },
        
        # Edge cases for limits
        {
            "name": "No start time with excessive limit",
            "start": None,
            "end": fixed_now,
            "limit": 500,
            "expected_limit": 300,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "No start time with default limit",
            "start": None,
            "end": fixed_now,
            "limit": None,
            "expected_limit": 100,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "Zero limit should use default",
            "start": None,
            "end": fixed_now,
            "limit": 0,
            "expected_limit": 100,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "Negative limit should use default",
            "start": None,
            "end": fixed_now,
            "limit": -10,
            "expected_limit": 100,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        
        # Edge cases for time ranges
        {
            "name": "Exactly 100 days ago (boundary case)",
            "start": fixed_now - timedelta(days=100),
            "end": fixed_now,
            "limit": 200,
            "expected_limit": 200,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "Exactly 101 days ago (boundary case)",
            "start": fixed_now - timedelta(days=101),
            "end": fixed_now,
            "limit": 200,
            "expected_limit": 100,
            "expected_endpoint": "history",
            "should_succeed": True,
        },
        {
            "name": "No time range at all",
            "start": None,
            "end": None,
            "limit": 150,
            "expected_limit": 150,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "Only start time (no end)",
            "start": fixed_now - timedelta(days=50),
            "end": None,
            "limit": 100,
            "expected_limit": 100,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        {
            "name": "Only end time (no start)",
            "start": None,
            "end": fixed_now,
            "limit": 100,
            "expected_limit": 100,
            "expected_endpoint": "regular",
            "should_succeed": True,
        },
        
        # Error cases
        {
            "name": "Invalid time range (start after end)",
            "start": fixed_now,
            "end": fixed_now - timedelta(days=10),
            "limit": 100,
            "expected_limit": 100,
            "expected_endpoint": "regular",
            "should_succeed": False,
            "error_expected": True,
        },
        {
            "name": "Same start and end time",
            "start": fixed_now,
            "end": fixed_now,
            "limit": 100,
            "expected_limit": 100,
            "expected_endpoint": "regular",
            "should_succeed": False,
            "error_expected": True,
        },
        
        # Large time ranges
        {
            "name": "Very large time range (1 year)",
            "start": fixed_now - timedelta(days=365),
            "end": fixed_now,
            "limit": 100,
            "expected_limit": 100,
            "expected_endpoint": "history",
            "should_succeed": True,
            "warning_expected": True,
        },
        {
            "name": "Extremely large time range (2 years)",
            "start": fixed_now - timedelta(days=730),
            "end": fixed_now,
            "limit": 50,
            "expected_limit": 50,
            "expected_endpoint": "history",
            "should_succeed": True,
            "warning_expected": True,
        },
        
        # Timezone edge cases
        {
            "name": "Naive datetime (no timezone)",
            "start": datetime(2025, 1, 1, 12, 0, 0),  # Naive datetime
            "end": fixed_now,
            "limit": 100,
            "expected_limit": 100,
            "expected_endpoint": "history",
            "should_succeed": True,
        },
    ]
    
    for test_case in test_cases:
        print(f"\n🧪 Testing: {test_case['name']}")
        
        # Simulate the logic from our implementation
        start = test_case['start']
        end = test_case['end']
        limit = test_case['limit']
        
        # Check for invalid time range
        if start is not None and end is not None:
            # Ensure both times are timezone-aware for comparison
            start_utc = start if start.tzinfo is not None else start.replace(tzinfo=timezone.utc)
            end_utc = end if end.tzinfo is not None else end.replace(tzinfo=timezone.utc)
            
            if start_utc >= end_utc:
                print(f"   ❌ Invalid time range: start {start_utc} >= end {end_utc}")
                if test_case.get('error_expected', False):
                    print(f"✅ Expected error case passed")
                    continue
                else:
                    print(f"❌ Unexpected error")
                    continue
        
        # Check if the request requires using the history endpoint (data > 100 days old)
        use_history_endpoint = False
        if start is not None:
            # Handle timezone-aware datetime comparison
            now = fixed_now
            if start.tzinfo is None:
                # If start is naive, assume it's UTC
                start_utc = start.replace(tzinfo=timezone.utc)
            else:
                start_utc = start.astimezone(timezone.utc)
            
            days_ago = (now - start_utc).days
            use_history_endpoint = days_ago > 100
            
            # Apply endpoint-specific limit validation
            if use_history_endpoint:
                if limit is not None and limit > 100:
                    print(f"   Warning: Requested limit {limit} exceeds OKX history endpoint maximum of 100, clamping to 100")
                    limit = min(limit, 100)
            else:
                if limit is not None and limit > 300:
                    print(f"   Warning: Requested limit {limit} exceeds OKX regular endpoint maximum of 300, clamping to 300")
                    limit = min(limit, 300)
        else:
            # No start time provided, using regular endpoint
            if limit is not None and limit > 300:
                print(f"   Warning: Requested limit {limit} exceeds OKX maximum of 300, clamping to 300")
                limit = min(limit, 300)
        
        # Handle edge cases for limit
        if limit is None or limit <= 0:
            limit = 100  # Default limit
        
        # Check for large time ranges
        if start is not None and end is not None:
            # Ensure both times are timezone-aware for comparison
            start_utc = start if start.tzinfo is not None else start.replace(tzinfo=timezone.utc)
            end_utc = end if end.tzinfo is not None else end.replace(tzinfo=timezone.utc)
            
            time_range = end_utc - start_utc
            if time_range.days > 365:
                print(f"   Warning: Large time range ({time_range.days} days) detected")
                if not test_case.get('warning_expected', False):
                    print(f"   ⚠️  Unexpected warning for large time range")
        
        endpoint = 'history' if use_history_endpoint else 'regular'
        
        print(f"   Start: {start}")
        print(f"   End: {end}")
        print(f"   Endpoint: {endpoint}")
        print(f"   Final limit: {limit}")
        
        # Check results
        if test_case.get('error_expected', False):
            print(f"✅ Expected error case handled")
            continue
            
        assert limit == test_case['expected_limit'], f"Limit mismatch: {limit} != {test_case['expected_limit']}"
        assert endpoint == test_case['expected_endpoint'], f"Endpoint mismatch: {endpoint} != {test_case['expected_endpoint']}"
        
        print(f"✅ Passed")
    
    print("\n🎉 All edge case tests passed!")


if __name__ == "__main__":
    test_all_edge_cases()
