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
Enhanced Binance execution client with Trade WebSocket support.

This module provides execution clients that use WebSocket Trade API
for order operations, providing lower latency and better performance for high-frequency trading.
"""

import asyncio
from typing import Optional

from nautilus_trader.adapters.binance.common.enums import BinanceAccountType
from nautilus_trader.adapters.binance.config import BinanceExecClientConfig
from nautilus_trader.adapters.binance.execution import BinanceCommonExecutionClient
from nautilus_trader.adapters.binance.futures.execution import BinanceFuturesExecutionClient
from nautilus_trader.adapters.binance.spot.execution import BinanceSpotExecutionClient
from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWSClient
from nautilus_trader.adapters.binance.websocket.trade_client import BinanceTradeWebSocketError
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.model.orders import LimitOrder
from nautilus_trader.model.orders import MarketOrder
from nautilus_trader.model.orders import Order
from nautilus_trader.model.orders import StopLimitOrder


class BinanceExecutionClientWS(BinanceCommonExecutionClient):

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client,
        account,
        market, 
        user,
        enum_parser,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider: InstrumentProvider,
        account_type: BinanceAccountType,
        base_url_ws: str,
        name: str | None,
        config: BinanceExecClientConfig,
    ) -> None:
        super().__init__(
            loop=loop,
            client=client,
            account=account,
            market=market,
            user=user,
            enum_parser=enum_parser,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
            account_type=account_type,
            base_url_ws=base_url_ws,
            name=name,
            config=config,
        )
        
        self._use_trade_websocket = config.use_trade_websocket
        self._trade_ws_client: Optional[BinanceTradeWSClient] = None
        
        if self._use_trade_websocket:
            self._log.info("Trade WebSocket API enabled for order operations")
            self._initialize_trade_websocket()
        else:
            self._log.info("WebSocket API is disabled")

    def _initialize_trade_websocket(self) -> None:
        try:
            self._trade_ws_client = BinanceTradeWSClient(
                clock=self._clock,
                api_key=self._http_client.api_key,
                api_secret=self._http_client._secret,
                testnet=self._use_trade_websocket,
                execution_report_handler=self._handle_trade_ws_execution_report
            )
            self._log.info("Trade WebSocket client initialized")
        except Exception as e:
            self._log.error(f"Failed to initialize Trade WebSocket client: {e}")
            self._use_trade_websocket = False
            self._trade_ws_client = None

    def _handle_trade_ws_execution_report(self, report: dict) -> None:
        """
        Handle execution report from Trade WebSocket and convert to Nautilus events.
        
        This method processes execution reports received from the Binance Trade WebSocket
        and converts them into appropriate Nautilus events like OrderAccepted, OrderFilled, etc.
        """
        try:
            self._log.debug(f"Processing execution report: {report}")
            
            symbol = report.get("symbol", "")
            client_order_id_str = report.get("clientOrderId", "")
            order_id = report.get("orderId", 0)
            execution_type = report.get("executionType", "")
            event_time = report.get("eventTime", 0)

            if not symbol or not client_order_id_str:
                self._log.warning(f"Missing required fields in execution report: {report}")
                return
            
            from nautilus_trader.model.identifiers import ClientOrderId, VenueOrderId
            from nautilus_trader.model.identifiers import InstrumentId
            from nautilus_trader.core.datetime import millis_to_nanos
            
            client_order_id = ClientOrderId(client_order_id_str)
            venue_order_id = VenueOrderId(str(order_id))
            instrument_id = self._get_cached_instrument_id(symbol)
            ts_event = millis_to_nanos(event_time)
            
            strategy_id = self._cache.strategy_id_for_order(client_order_id)
            
            if strategy_id is None:
                self._log.debug(f"No strategy ID found for order {client_order_id}, sending status report")
                self._send_execution_report_as_status_report(report, instrument_id, client_order_id, venue_order_id, ts_event)
                return
            
            if execution_type == "NEW":
                self._handle_order_accepted(strategy_id, instrument_id, client_order_id, venue_order_id, ts_event)
            elif execution_type == "TRADE":
                self._handle_order_filled(report, strategy_id, instrument_id, client_order_id, venue_order_id, ts_event)
            elif execution_type == "CANCELED":
                self._handle_order_canceled(strategy_id, instrument_id, client_order_id, venue_order_id, ts_event)
            elif execution_type == "REJECTED":
                self._handle_order_rejected(strategy_id, instrument_id, client_order_id, venue_order_id, ts_event, report)
            else:
                self._log.debug(f"Unhandled execution type: {execution_type}")
                
        except Exception as e:
            self._log.error(f"Error processing Trade WebSocket execution report: {e}")
            self._log.exception(e)
    
    def _handle_order_accepted(self, strategy_id, instrument_id, client_order_id, venue_order_id, ts_event):
        """Handle NEW execution type - order accepted."""
        self.generate_order_accepted(
            strategy_id=strategy_id,
            instrument_id=instrument_id,
            client_order_id=client_order_id,
            venue_order_id=venue_order_id,
            ts_event=ts_event,
        )
    
    def _handle_order_filled(self, report, strategy_id, instrument_id, client_order_id, venue_order_id, ts_event):
        """Handle TRADE execution type - order filled."""
        try:
            from nautilus_trader.model.objects import Price, Quantity, Money
            from nautilus_trader.model.identifiers import TradeId
            from nautilus_trader.model.enums import LiquiditySide
            
            last_qty = report.get("lastExecutedQuantity", "0")
            last_px = report.get("lastExecutedPrice", "0")
            side = report.get("side", "")
            order_type = report.get("orderType", "")
            commission_amount = report.get("commissionAmount", "0")
            commission_asset = report.get("commissionAsset", "")
            trade_id = report.get("tradeId", 0)
            is_maker = report.get("isMakerSide", False)
            
            instrument = self._instrument_provider.find(instrument_id=instrument_id)
            if not instrument:
                self._log.error(f"Instrument not found: {instrument_id}")
                return
            
            last_quantity = Quantity.from_str(last_qty)
            last_price = Price.from_str(last_px)
            
            if commission_asset and commission_amount != "0":
                commission = Money.from_str(f"{commission_amount} {commission_asset}")
            else:
                commission = Money(0, instrument.base_currency)
            
            self.generate_order_filled(
                strategy_id=strategy_id,
                instrument_id=instrument_id,
                client_order_id=client_order_id,
                venue_order_id=venue_order_id,
                venue_position_id=None,  # NETTING accounts
                trade_id=TradeId(str(trade_id)),
                order_side=self._enum_parser.parse_binance_order_side(side),
                order_type=self._enum_parser.parse_binance_order_type(order_type),
                last_qty=last_quantity,
                last_px=last_price,
                quote_currency=instrument.quote_currency,
                commission=commission,
                liquidity_side=LiquiditySide.MAKER if is_maker else LiquiditySide.TAKER,
                ts_event=ts_event,
            )
            
        except Exception as e:
            self._log.error(f"Error handling order filled: {e}")
    
    def _handle_order_canceled(self, strategy_id, instrument_id, client_order_id, venue_order_id, ts_event):
        self.generate_order_canceled(
            strategy_id=strategy_id,
            instrument_id=instrument_id,
            client_order_id=client_order_id,
            venue_order_id=venue_order_id,
            ts_event=ts_event,
        )
    
    def _handle_order_rejected(self, strategy_id, instrument_id, client_order_id, venue_order_id, ts_event, report):
        reason = report.get("rejectReason", "Unknown rejection reason")
        self.generate_order_rejected(
            strategy_id=strategy_id,
            instrument_id=instrument_id,
            client_order_id=client_order_id,
            venue_order_id=venue_order_id,
            reason=reason,
            ts_event=ts_event,
        )
    
    def _send_execution_report_as_status_report(self, report, instrument_id, client_order_id, venue_order_id, ts_event):
        """
        Send execution report as order status report when no strategy ID is found.
        
        This handles execution reports for orders that are not tracked by any strategy,
        typically from external order management or manual trading.
        """
        try:
            from nautilus_trader.model.events import OrderStatusReport
            from nautilus_trader.model.objects import Price, Quantity
            from nautilus_trader.model.enums import OrderStatus, OrderSide, OrderType, TimeInForce
            from nautilus_trader.model.enums import TriggerType, TrailingOffsetType
            from nautilus_trader.core.uuid import UUID4
            from decimal import Decimal
            
            order_side_str = report.get("side", "")
            order_type_str = report.get("orderType", "")
            time_in_force_str = report.get("timeInForce", "")
            order_status_str = report.get("orderStatus", "")
            price_str = report.get("price", "0")
            stop_price_str = report.get("stopPrice", "0")
            quantity_str = report.get("quantity", "0")
            filled_qty_str = report.get("cumulativeFilledQuantity", "0")
            avg_price_str = report.get("avgPrice", "0")
            reduce_only = report.get("reduceOnly", False)
            
            order_side = self._enum_parser.parse_binance_order_side(order_side_str)
            order_type = self._enum_parser.parse_binance_order_type(order_type_str)
            time_in_force = self._enum_parser.parse_binance_time_in_force(time_in_force_str)
            order_status = self._enum_parser.parse_binance_order_status(order_status_str)
            
            price = Price.from_str(price_str) if price_str and price_str != "0" else None
            trigger_price = Price.from_str(stop_price_str) if stop_price_str and stop_price_str != "0" else None
            quantity = Quantity.from_str(quantity_str)
            filled_qty = Quantity.from_str(filled_qty_str)
            avg_px = Decimal(avg_price_str) if avg_price_str and avg_price_str != "0" else None
            
            trigger_type = TriggerType.NO_TRIGGER
            if trigger_price and trigger_price > 0:
                trigger_type = TriggerType.LAST_PRICE
            
            post_only = time_in_force_str == "GTX"
            
            report_obj = OrderStatusReport(
                account_id=self.account_id,
                instrument_id=instrument_id,
                client_order_id=client_order_id,
                venue_order_id=venue_order_id,
                order_side=order_side,
                order_type=order_type,
                time_in_force=time_in_force,
                order_status=order_status,
                price=price,
                trigger_price=trigger_price,
                trigger_type=trigger_type,
                trailing_offset=None,
                trailing_offset_type=TrailingOffsetType.NO_TRAILING_OFFSET,
                quantity=quantity,
                filled_qty=filled_qty,
                avg_px=avg_px,
                post_only=post_only,
                reduce_only=reduce_only,
                report_id=UUID4(),
                ts_accepted=ts_event,
                ts_last=ts_event,
                ts_init=self._clock.timestamp_ns(),
            )
            
            self._send_order_status_report(report_obj)
            self._log.info(f"Sent OrderStatusReport for untracked order {client_order_id}")
        except Exception as e:
            self._log.error(f"Error creating OrderStatusReport from execution report: {e}")
            self._log.error(f"Report data: {report}")
            self._log.exception(e)

    async def _connect(self) -> None:
        await super()._connect()
        
        if self._use_trade_websocket and self._trade_ws_client:
            try:
                await self._trade_ws_client.connect()
                self._log.info("Trade WebSocket connected successfully")
            except BinanceTradeWebSocketError as e:
                self._log.error(f"Failed to connect Trade WebSocket: {e}")
                self._use_trade_websocket = False
                self._trade_ws_client = None

    async def _disconnect(self) -> None:
        if self._trade_ws_client:
            try:
                await self._trade_ws_client.disconnect()
                self._log.info("Trade WebSocket disconnected")
            except Exception as e:
                self._log.error(f"Error disconnecting Trade WebSocket: {e}")
        await super()._disconnect()

    async def _submit_market_order(
        self,
        order: MarketOrder,
        position_side=None,
    ) -> None:
        if self._use_trade_websocket and self._trade_ws_client:
            await self._submit_order_via_websocket(order, position_side)
        else:
            await super()._submit_market_order(order, position_side)

    async def _submit_limit_order(
        self,
        order: LimitOrder,
        position_side=None,
    ) -> None:
        if self._use_trade_websocket and self._trade_ws_client:
            await self._submit_order_via_websocket(order, position_side)
        else:
            await super()._submit_limit_order(order, position_side)

    async def _submit_stop_limit_order(
        self,
        order: StopLimitOrder,
        position_side=None,
    ) -> None:
        if self._use_trade_websocket and self._trade_ws_client:
            await self._submit_order_via_websocket(order, position_side)
        else:
            await super()._submit_stop_limit_order(order, position_side)

    async def _submit_order_via_websocket(self, order: Order, position_side=None) -> None:
        try:
            symbol = order.instrument_id.symbol.value
            side = self._enum_parser.parse_internal_order_side(order.side)
            order_type = self._enum_parser.parse_internal_order_type(order)
            
            kwargs = {
                "symbol": symbol,
                "side": side,
                "order_type": order_type,
                "new_client_order_id": order.client_order_id.value,
            }
            
            if hasattr(order, 'quantity') and order.quantity is not None:
                kwargs["quantity"] = str(order.quantity)
            
            if hasattr(order, 'price') and order.price is not None:
                kwargs["price"] = str(order.price)
                
            if hasattr(order, 'trigger_price') and order.trigger_price is not None:
                kwargs["stop_price"] = str(order.trigger_price)
            
            if hasattr(order, 'time_in_force'):
                time_in_force = self._determine_time_in_force(order)
                if time_in_force:
                    kwargs["time_in_force"] = time_in_force.value
            
            if position_side:
                kwargs["position_side"] = position_side.value
            
            reduce_only = self._determine_reduce_only_str(order)
            if reduce_only:
                kwargs["reduce_only"] = reduce_only == "true"
            
            self._log.debug(f"Submitting order via WebSocket: {kwargs}")
            response = await self._trade_ws_client.place_order(**kwargs)
            
            self._log.info(f"Order submitted via WebSocket: {response['orderId']}")
            
        except Exception as e:
            self._log.error(f"Failed to submit order via WebSocket: {e}")
            # Fall back to HTTP submission
            self._log.warning("Falling back to HTTP submission")
            if isinstance(order, MarketOrder):
                await super()._submit_market_order(order, position_side)
            elif isinstance(order, LimitOrder):
                await super()._submit_limit_order(order, position_side)
            elif isinstance(order, StopLimitOrder):
                await super()._submit_stop_limit_order(order, position_side)

    async def _cancel_order(self, order) -> None:
        if self._use_trade_websocket and self._trade_ws_client:
            try:
                symbol = order.instrument_id.symbol.value
                
                response = await self._trade_ws_client.cancel_order(
                    symbol=symbol,
                    orig_client_order_id=order.client_order_id.value,
                )
                
                self._log.info(f"Order cancelled via WebSocket: {response['orderId']}")
                
            except Exception as e:
                self._log.error(f"Failed to cancel order via WebSocket: {e}")
                await super()._cancel_order(order)
        else:
            await super()._cancel_order(order)

    async def _modify_order(self, order, quantity: str = None, price: str = None) -> None:
        if self._use_trade_websocket and self._trade_ws_client:
            try:
                symbol = order.instrument_id.symbol.value
                side = self._enum_parser.parse_internal_order_side(order.side)
                
                response = await self._trade_ws_client.modify_order(
                    symbol=symbol,
                    side=side,
                    orig_client_order_id=order.client_order_id.value,
                    quantity=quantity,
                    price=price,
                )
                
                self._log.info(f"Order modified via WebSocket: {response['orderId']}")
                
            except Exception as e:
                self._log.error(f"Failed to modify order via WebSocket: {e}")
                await super()._modify_order(order, quantity, price)
        else:
            await super()._modify_order(order, quantity, price)


class BinanceSpotExecutionClientWS(BinanceSpotExecutionClient):

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider,
        base_url_ws: str,
        config: BinanceExecClientConfig,
        account_type: BinanceAccountType = BinanceAccountType.SPOT,
        name: str | None = None,
    ) -> None:
        if config.use_trade_websocket:
            from nautilus_trader.adapters.binance.spot.http import BinanceSpotAccountHttpAPI
            from nautilus_trader.adapters.binance.spot.http import BinanceSpotMarketHttpAPI
            from nautilus_trader.adapters.binance.spot.http import BinanceSpotUserDataHttpAPI
            from nautilus_trader.adapters.binance.spot.enums import BinanceSpotEnumParser
            
            spot_http_account = BinanceSpotAccountHttpAPI(client, clock, account_type)
            spot_http_market = BinanceSpotMarketHttpAPI(client, account_type)
            spot_http_user = BinanceSpotUserDataHttpAPI(client, account_type)
            spot_enum_parser = BinanceSpotEnumParser()
            
            BinanceExecutionClientWS.__init__(
                self,
                loop=loop,
                client=client,
                account=spot_http_account,
                market=spot_http_market,
                user=spot_http_user,
                enum_parser=spot_enum_parser,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=instrument_provider,
                account_type=account_type,
                base_url_ws=base_url_ws,
                name=name,
                config=config,
            )
        else:
            super().__init__(
                loop=loop,
                client=client,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=instrument_provider,
                base_url_ws=base_url_ws,
                config=config,
                account_type=account_type,
                name=name,
            )


class BinanceFuturesExecutionClientWS(BinanceFuturesExecutionClient):

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider,
        base_url_ws: str,
        config: BinanceExecClientConfig,
        account_type: BinanceAccountType = BinanceAccountType.USDT_FUTURE,
        name: str | None = None,
    ) -> None:
        if config.use_trade_websocket:
            from nautilus_trader.adapters.binance.futures.http import BinanceFuturesAccountHttpAPI
            from nautilus_trader.adapters.binance.futures.http import BinanceFuturesMarketHttpAPI
            from nautilus_trader.adapters.binance.futures.http import BinanceFuturesUserDataHttpAPI
            from nautilus_trader.adapters.binance.futures.enums import BinanceFuturesEnumParser
            
            futures_http_account = BinanceFuturesAccountHttpAPI(client, clock, account_type)
            futures_http_market = BinanceFuturesMarketHttpAPI(client, account_type)
            futures_http_user = BinanceFuturesUserDataHttpAPI(client, account_type)
            futures_enum_parser = BinanceFuturesEnumParser()
            
            BinanceExecutionClientWS.__init__(
                self,
                loop=loop,
                client=client,
                account=futures_http_account,
                market=futures_http_market,
                user=futures_http_user,
                enum_parser=futures_enum_parser,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=instrument_provider,
                account_type=account_type,
                base_url_ws=base_url_ws,
                name=name,
                config=config,
            )
        else:
            super().__init__(
                loop=loop,
                client=client,
                msgbus=msgbus,
                cache=cache,
                clock=clock,
                instrument_provider=instrument_provider,
                base_url_ws=base_url_ws,
                config=config,
                account_type=account_type,
                name=name,
            )