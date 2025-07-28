"""
Binance WebSocket vs REST API Latency Benchmark

This comprehensive test compares latency between:
1. REST API (requires HMAC-SHA256 signature per request)  
2. WebSocket with Ed25519 session.logon (signature-free after authentication)

The test demonstrates the performance benefits of Ed25519 session authentication
by measuring actual network latency for order status requests.

Required Environment Variables:
- BINANCE_API_KEY: Your Binance API key
- BINANCE_API_SECRET: Your Binance API secret (for REST HMAC signing)
- BINANCE_ED25519_PRIVATE_KEY: Your Ed25519 private key (for WebSocket session auth)
"""

import asyncio
import hashlib
import hmac
import os
import statistics
import time
import urllib.parse
from dataclasses import dataclass
from typing import List, Optional

import aiohttp

from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWSClient
from nautilus_trader.common.component import LiveClock


@dataclass
class LatencyResult:
    method: str
    operation: str
    latencies_ms: List[float]
    mean_ms: float
    median_ms: float
    p95_ms: float
    p99_ms: float
    min_ms: float
    max_ms: float
    std_ms: float
    success_rate: float


class BinanceRestClient:

    def __init__(self, api_key: str, api_secret: str, testnet: bool = True):
        self.api_key = api_key
        self.api_secret = api_secret
        self.base_url = (
            "https://testnet.binancefuture.com" if testnet 
            else "https://fapi.binance.com"
        )
        self.session: Optional[aiohttp.ClientSession] = None
        
    async def __aenter__(self):
        self.session = aiohttp.ClientSession()
        return self
        
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            await self.session.close()
            
    def _generate_signature(self, query_string: str) -> str:
        return hmac.new(
            self.api_secret.encode('utf-8'),
            query_string.encode('utf-8'),
            hashlib.sha256
        ).hexdigest()
        
    async def get_order_status(self, symbol: str, orig_client_order_id: str) -> dict:
        timestamp = int(time.time() * 1000)
        
        params = {
            "symbol": symbol,
            "origClientOrderId": orig_client_order_id,
            "timestamp": timestamp
        }
        
        query_string = urllib.parse.urlencode(params)
        signature = self._generate_signature(query_string)
        
        params["signature"] = signature
        
        headers = {"X-MBX-APIKEY": self.api_key}
        
        async with self.session.get(
            f"{self.base_url}/fapi/v1/order",
            params=params,
            headers=headers
        ) as response:
            return await response.json()


class BinanceLatencyTester:

    def __init__(self):
        self.api_key = os.getenv("BINANCE_API_KEY")
        self.api_secret = os.getenv("BINANCE_API_SECRET")
        self.ed25519_private_key = os.getenv("BINANCE_ED25519_PRIVATE_KEY")
        
        missing_vars = []
        if not self.api_key:
            missing_vars.append("BINANCE_API_KEY")
        if not self.api_secret:
            missing_vars.append("BINANCE_API_SECRET")
        if not self.ed25519_private_key:
            missing_vars.append("BINANCE_ED25519_PRIVATE_KEY")
            
        if missing_vars:
            raise ValueError(
                f"Missing required environment variables: {', '.join(missing_vars)}\n"
                "Please set all required variables before running the test."
            )
        
        self.testnet = True
        self.test_iterations = 30
        self.symbol = "BTCUSDT"
        self.test_order_id = "nonexistent_order_12345"
        
        self.clock = LiveClock()
        self.ws_client: Optional[BinanceTradeWSClient] = None
        
    async def setup_websocket(self) -> None:
        print("🔌 Setting up WebSocket client...")
        
        self.ws_client = BinanceTradeWSClient(
            clock=self.clock,
            api_key=self.api_key,
            ed25519_private_key=self.ed25519_private_key,
            testnet=self.testnet
        )
        
        await self.ws_client.connect()
        
        max_wait_seconds = 15
        wait_time = 0
        while not await self.ws_client.is_ready() and wait_time < max_wait_seconds:
            await asyncio.sleep(0.1)
            wait_time += 0.1
            
        if not await self.ws_client.is_ready():
            raise RuntimeError(
                f"WebSocket authentication failed after {max_wait_seconds}s. "
                "Check your credentials and network connection."
            )
            
        print("✅ WebSocket authenticated and ready (Ed25519 session established)")
        
    async def cleanup_websocket(self) -> None:
        if self.ws_client:
            await self.ws_client.disconnect()
            
    async def test_websocket_latency(self) -> LatencyResult:
        print(f"🚀 Testing WebSocket latency ({self.test_iterations} iterations)...")
        
        latencies = []
        success_count = 0
        
        for i in range(self.test_iterations):
            start_time = time.perf_counter()
            
            try:
                await self.ws_client.get_order_status(
                    symbol=self.symbol,
                    order_id=None,
                    orig_client_order_id=self.test_order_id
                )
                success_count += 1
            except Exception:
                pass
                
            end_time = time.perf_counter()
            latency_ms = (end_time - start_time) * 1000
            latencies.append(latency_ms)
            
            if i < self.test_iterations - 1:
                await asyncio.sleep(0.02)
                
        return self._calculate_statistics(
            latencies, 
            "WebSocket (Ed25519 Session)", 
            "Order Status Query",
            success_count
        )
        
    async def test_rest_latency(self) -> LatencyResult:
        print(f"🌐 Testing REST API latency ({self.test_iterations} iterations)...")
        
        latencies = []
        success_count = 0
        
        async with BinanceRestClient(self.api_key, self.api_secret, self.testnet) as rest_client:
            for i in range(self.test_iterations):
                start_time = time.perf_counter()
                
                try:
                    await rest_client.get_order_status(
                        symbol=self.symbol,
                        orig_client_order_id=self.test_order_id
                    )
                    success_count += 1
                except Exception:
                    pass
                    
                end_time = time.perf_counter()
                latency_ms = (end_time - start_time) * 1000
                latencies.append(latency_ms)
                
                # Brief pause between requests
                if i < self.test_iterations - 1:
                    await asyncio.sleep(0.02)
                    
        return self._calculate_statistics(
            latencies, 
            "REST API (HMAC-SHA256)", 
            "Order Status Query",
            success_count
        )
        
    def _calculate_statistics(
        self, 
        latencies: List[float], 
        method: str, 
        operation: str,
        success_count: int
    ) -> LatencyResult:
        return LatencyResult(
            method=method,
            operation=operation,
            latencies_ms=latencies,
            mean_ms=statistics.mean(latencies),
            median_ms=statistics.median(latencies),
            p95_ms=self._percentile(latencies, 95),
            p99_ms=self._percentile(latencies, 99),
            min_ms=min(latencies),
            max_ms=max(latencies),
            std_ms=statistics.stdev(latencies) if len(latencies) > 1 else 0.0,
            success_rate=(success_count / len(latencies)) * 100
        )
        
    def _percentile(self, data: List[float], percentile: float) -> float:
        """Calculate percentile of sorted data."""
        sorted_data = sorted(data)
        index = int((percentile / 100) * len(sorted_data))
        return sorted_data[min(index, len(sorted_data) - 1)]
        
    def print_comprehensive_results(self, results: List[LatencyResult]) -> None:
        print("\n" + "="*85)
        print("🏆 BINANCE WEBSOCKET vs REST API LATENCY BENCHMARK RESULTS")
        print("="*85)
        
        for result in results:
            print(f"\n📊 {result.method}")
            print(f"   Operation:     {result.operation}")
            print(f"   Mean Latency:  {result.mean_ms:.2f} ms")
            print(f"   Median:        {result.median_ms:.2f} ms")
            print(f"   P95:           {result.p95_ms:.2f} ms")
            print(f"   P99:           {result.p99_ms:.2f} ms")
            print(f"   Min:           {result.min_ms:.2f} ms")
            print(f"   Max:           {result.max_ms:.2f} ms")
            print(f"   Std Dev:       {result.std_ms:.2f} ms")
            print(f"   Samples:       {len(result.latencies_ms)}")
            print(f"   Success Rate:  {result.success_rate:.1f}%")
            
        if len(results) >= 2:
            ws_result = next(r for r in results if "WebSocket" in r.method)
            rest_result = next(r for r in results if "REST" in r.method)
            
            self._print_performance_analysis(ws_result, rest_result)
            
        self._print_technical_insights()
        
    def _print_performance_analysis(self, ws_result: LatencyResult, rest_result: LatencyResult) -> None:
        improvement = ((rest_result.mean_ms - ws_result.mean_ms) / rest_result.mean_ms) * 100
        latency_diff = rest_result.mean_ms - ws_result.mean_ms
        
        print(f"\n🚀 PERFORMANCE ANALYSIS")
        print(f"   Speed Improvement:     {improvement:.1f}% faster")
        print(f"   Absolute Difference:   {latency_diff:.2f} ms")
        print(f"   WebSocket Mean:        {ws_result.mean_ms:.2f} ms")
        print(f"   REST API Mean:         {rest_result.mean_ms:.2f} ms")
        
        if improvement > 10:
            print(f"   💡 Significant improvement! In high-frequency trading,")
            print(f"      {latency_diff:.1f}ms per request can mean better fill prices.")
        elif improvement > 0:
            print(f"   ✅ WebSocket shows measurable latency advantage.")
        else:
            print(f"   ⚠️  No significant improvement detected. Check network conditions.")
            
    def _print_technical_insights(self) -> None:
        print(f"\n🔑 TECHNICAL INSIGHTS")
        print(f"   • WebSocket Advantages:")
        print(f"     - Ed25519 session.logon eliminates per-request signature generation")
        print(f"     - Persistent connection avoids TCP handshake overhead")
        print(f"     - Binary WebSocket protocol vs HTTP text overhead")
        print(f"     - Reduced TLS negotiation (connection reuse)")
        print(f"   ")
        print(f"   • REST API Overhead:")
        print(f"     - HMAC-SHA256 signature calculation for every request")
        print(f"     - HTTP connection establishment (even with keep-alive)")
        print(f"     - HTTP headers and protocol overhead")
        print(f"     - Request/response parsing overhead")
        print(f"   ")
        print(f"   • Trading Impact:")
        print(f"     - Lower latency = better execution prices")
        print(f"     - Reduced market impact in high-frequency strategies")
        print(f"     - More responsive risk management")
        print("="*85)


async def main():
    print("🔧 Initializing Binance WebSocket vs REST API Latency Benchmark")
    print("   This test measures the performance benefits of Ed25519 session authentication\n")
    
    tester = BinanceLatencyTester()
    
    try:
        await tester.setup_websocket()
        
        print(f"\n⏱️  Running comparative latency analysis...")
        
        ws_result = await tester.test_websocket_latency()
        rest_result = await tester.test_rest_latency()
        
        tester.print_comprehensive_results([ws_result, rest_result])
        
    except Exception as e:
        print(f"❌ Benchmark failed: {e}")
        print("   Please check your credentials and network connection.")
        raise
    finally:
        await tester.cleanup_websocket()
        print("\n🏁 Benchmark completed successfully!")


if __name__ == "__main__":
    asyncio.run(main()) 