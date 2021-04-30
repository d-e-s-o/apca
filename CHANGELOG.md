Unreleased
----------
- Added `ApiInfo::from_parts` constructor
- Bumped minimum supported Rust version to `1.46`


0.18.0
------
- Introduced trailing stop order types
- Added support for paging of account activity data
- Added support for specifying reported account activity direction as
  well as `until` and `after` times
- Bumped `time-util` dependency to `0.3`


0.17.0
------
- Added `PendingReplace` variant to `TradeStatus` enum
- Added support for listing nested orders
- Replaced usage of private `serde` API with inline code
- Bumped minimum supported Rust version to `1.44`
- Excluded unnecessary files from being contained in release bundle
- Replaced `async-tungstenite` dependency with `tokio-tungstenite`
- Removed `chrono` dependency
- Bumped `hyper` dependency to `0.14`
- Bumped `hyper-tls` dependency to `0.5`
- Bumped `tokio` dependency to `1.0`
- Bumped `websocket-util` dependency to `0.7`


0.16.0
------
- Converted `NonTradeActivity::quantity` from `u64` to `Num`
- Bumped `http-endpoint` dependency to `0.2`
- Bumped `websocket-util` dependency to `0.6`
- Bumped `async-tungstenite` dependency to `0.8`
- Bumped `serde_urlencoded` dependency to `0.7`


0.15.0
------
- Enabled CI pipeline comprising building, testing, linting, and
  coverage collection of the project
  - Added badges indicating pipeline status and code coverage percentage
- Bumped `websocket-util` dependency to `0.5`
- Bumped `async-tungstenite` dependency to `0.5`


0.14.0
------
- Added example illustrating how to submit a limit order
- Bumped `http-endpoint` dependency to `0.2`


0.13.0
------
- Added `stream_raw` function for interfacing with raw event streams
- Adjusted streaming function to expect reference to `ApiInfo` object
- Removed serialization support for account & trade events
- Removed `TradeStatus::to_order_status` method
- Bumped `websocket-util` dependency to `0.4`


0.12.0
------
- Added support for handling unknown variants for `account::Status`,
  `asset::Exchange`, `events::TradeStatus`, and `order::Status`
- Added `PendingReplace` order status variant
- Removed serialization support for `Exchange` enum


0.11.0
------
- Added support for bracket-style orders
- Added `From` implementation for `asset::Symbol` type
- Added support for almost-default construction of various request types
- Converted `Account::daytrade_count` to `u64`
- Decreased tracing verbosity by one level
- Bumped `num-decimal` dependency to `0.2`


0.10.0
------
- Added `average_fill_price` to `Order` type
- Fixed issue when deserializing non-trade activity object without a
  quantity


0.9.0
-----
- Added `quantity` field to `NonTradeActivity` type
- Added `ReplaceRejected` and `CancelRejected` variants to `TradeStatus`
  enum
- Use absolute values for quantity reported in `Position` objects


0.8.1
-----
- Added support for negating order and position `Side` types


0.8.0
-----
- Added support for associating client IDs with orders
- Converted various quantities from `Num` to `u64`
- Hooked up `order_id` field to `TradeActivity` type
- Bumped `time-util` dependency to `0.2`


0.7.0
-----
- Bumped `websocket-util` dependency to `0.3`


0.6.0
-----
- Added support for querying `/v2/account/activities` endpoint
- Added support for listing orders based on their status
- Introduced `Replaced` variant to `TradeStatus` enum


0.5.0
-----
- Added support for changing an existing order
- Introduced `TradeStatus::to_order_status` helper method
- Implemented `Eq` and `Hash` for the various `Id` types


0.4.0
-----
- Added support for opening and closing auction orders
- Factored out `time-util` crate


0.3.1
-----
- Added support for accessing `/v2/account/configurations` endpoint
- Added support for querying `/v1/bars/<timeframe>` endpoint
- Added support for serializing account & trade events
- Switched from using `log` to `tracing` as a logging/tracing provider
- Switched to using `serde_urlencoded` for encoding query parameters
- Bumped `http-endpoint` dependency to `0.1.1`


0.3.0
-----
- Added support for liquidating an existing position
- Added support for short selling
- Removed `AssetReq` and `PositionReq` types
- Bumped `async-tungstenite` dependency to `0.3`


0.2.2
-----
- Migrated streaming functionality from `websocket` to
  `async-tungstenite`
- Dropped dependency on `futures` `0.1`
- Factored out `http-endpoint` crate
- Factored out `websocket-util` crate
- Correctly implemented `std::error::Error::source` for `Error type
- Bumped `env_logger` dependency to `0.7`
- Bumped `uuid` dependency to `0.8`


0.2.1
-----
- Removed support for `v1` API
- Implemented `FromStr` for various `/v2/asset` types


0.2.0
-----
- Converted `api` functionality to use `async`/`await` syntax
- Bumped `test-env-log` dependency to `0.2`


0.1.1
-----
- Added support for accessing various `v2` endpoints:
  - `/v2/account`
  - `/v2/asset`
  - `/v2/orders`
  - `/v2/positions`
- Bumped `websocket` dependency to `0.24`


0.1.0
-----
- Initial release
