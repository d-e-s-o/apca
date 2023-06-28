0.27.2
------
- Expanded crate documentation with a high level overview
- Bumped `async-compression` dependency to `0.4`


0.27.1
------
- Exported `api::v2::updates::{Authenticate,Listen}` enums as part of
  unofficial unstable API


0.27.0
------
- Added support for overwriting default realtime data bar, quote, and
  trade types
- Added support for working with custom v2 realtime data streaming URLs
- Added `FillOrKill` and `ImmediateOrCancel` variants to
  `api::v2::order::TimeInForce` type
- Changed various `data::v2::stream::{Bar, Quote, Trade}` members from
  `u64` to `Num`
- Switched to using new stream authentication request message format
- Adjusted publish workflow to also create GitHub release and Git tag


0.26.2
------
- Introduced `vendored-openssl` to build with vendored `openssl` library
- Added GitHub Actions workflow for publishing the crate


0.26.1
------
- Made more types serializable/deserializable


0.26.0
------
- Added support for historic trade retrieval via `data::v2::trades`
- Adjusted `data::v2::last_quote` module to work with multiple symbols
  and renamed it to `last_quotes`
- Added `client_order_id` member to `api::v2::order::ChangeReq`
- Made `quantity` and `time_in_force` members of
  `api::v2::order::ChangeReq` optional
- Added `quantity_available` member to `api::v2::position::Position`
- Bumped minimum supported Rust version to `1.59`
- Bumped `websocket-util` dependency to `0.11`
- Bumped `tokio-tungstenite` dependency to `0.18`


0.25.1
------
- Added optional `price` member to
  `api::v2::account_activities::NonTradeActivity` type
- Switched to using GitHub Actions as CI provider
- Bumped minimum supported Rust version to `1.57`


0.25.0
------
- Added `gzip` compression support for transparent API response
  compression controlled by default enabled `gzip` feature
- Added support for subscribing to realtime trades
- Reworked symbols related types in `data::v2::stream` module
- Added `class` member to `api::v2::order::Order` type
- Added `symbols` member to `api::v2::orders::OrdersReq` type
- Added `Deserialize` implementation for more types
- Updated `ActivityType` enum to be in sync with upstream variants
- Made `api::v2::position::Position` exhaustive
- Bumped `uuid` dependency to `1.0`


0.24.0
------
- Made more types serializable/deserializable
- Renamed various `Trade*` types to `Order*`
- Removed `#[non_exhaustive]` attribute from various types


0.23.0
------
- Added support for subscribing to realtime quotes in addition to
  aggregate bar data
- Adjusted data API request types to include optional data feed to use
- Adjusted `data::v2::last_quote::Get` to accept a `LastQuoteReq` object
- Added `Crypto` variant to `api::v2::asset::Class` enum and made it
  non-exhaustive
- Added `Otc` variant to `api::v2::asset::Exchange` enum and made it
  non-exhaustive
- Introduced `api::v2::order::Status::is_terminal` method
- Replaced infallible `From<Into<String>>` conversion for
  `asset::Symbol` with fallible `TryFrom`
- Made members of `ApiInfo` publicly accessible and added more URL
  members
- Renamed various 422 HTTP status error variants to `InvalidInput`
- Changed various limit types to `usize`
- Changed `api::v2::orders::OrdersReq::limit` to be an `Option`
- Added example illustrating how to stream realtime market data


0.22.5
------
- Renamed `data::v2::bars::BarReq` to `BarsReq`
  - Deprecated `data::v2::bars::BarReq`
- Introduced `data::v2::bars::BarsReqInit` type
- Introduced `ApiInfo::into_parts` method


0.22.4
------
- Adjusted `Subscribable` trait to make all created futures implement
  `Send`


0.22.3
------
- Added `api::v2::calendar` module for retrieving historic and future
  market open and close times
- Added support for historic quote retrieval via `data::v2::quotes`


0.22.2
------
- Fixed JSON decoding error when no bars are returned in response to
  `data::v2::bars::Get` request


0.22.1
------
- Added support for realtime market data streaming via
  `data::v2::stream::RealtimeData`


0.22.0
------
- Reworked account update streaming support using a subscription based
  design
  - Renamed `api::v2::events` to `api::v2::updates`
  - Removed `event` module providing low-level access to update streaming
- Renamed `InsufficientFunds` variant of `api::v2::order::PostError` and
  `api::v2::order::PatchError` to `NotPermitted`
- Removed support for streaming account updates
- Removed `data::v2::stocks` module alias
- Switched from using `test-env-log` to `test-log`
- Bumped minimum supported Rust version to `1.56`
- Switched to using Rust 2021 Edition
- Added `async-trait` dependency in version `0.1.51`
- Bumped `websocket-util` dependency to `0.10.1`
- Bumped `tokio-tungstenite` dependency to `0.16`


0.21.1
------
- Introduced support for retrieving the last quote for a symbol
- Aliased `data::v2::bars` module to `data::v2::stocks`
  - Deprecated `data::v2::stocks` module alias


0.21.0
------
- Introduced support for historic data retrieval using v2 API
- Added bindings for watchlist management
- Added support for submitting notional orders
- Adjusted all quantities to be of type `Num` to fully support
  trading with fractional quantities
- Tagged more functions and methods as `#[inline]`
- Removed support for v1 historic data API


0.20.0
------
- Migrated most usages of `SystemTime` date times to `chrono::DateTime`
- Added `Activity::time` method for retrieving the time stamp of an
  account activity
- Marked several more types as `non_exhaustive`
- Made price related attributes of `Position` type optional after
  announcement of breaking API change at Alpaca
- Added `fractionable` attribute to `Asset` type
- Switched to using tarpaulin for code coverage collection
- Formatted code base using `rustfmt` and checked in configuration
  - Added enforcement of code formatting style checks in CI
- Added CI checks for auto generated code documentation
- Removed `time-util` dependency


0.19.0
------
- Added `ApiInfo::from_parts` constructor
- Adjusted `Client::issue` to accept request input via reference
- Introduced `ConversionError` type
  - Replaced `unwrap`s with proper error variants
- Updated `ActivityType` enum to be in sync with upstream variants
- Added support for unknown `ActivityType` variants
- Switched to using `thiserror` crate for defining error types
- Updated `num-decimal` to use version `0.4` of the `num-*` crates
- Bumped minimum supported Rust version to `1.46`
- Bumped `http-endpoint` dependency to `0.5`
- Bumped `websocket-util` dependency to `0.8`
- Bumped `tokio-tungstenite` dependency to `0.14`


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
- Bumped `http-endpoint` dependency to `0.4`
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
- Added support for querying `/v1/bars/{timeframe}` endpoint
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
- Correctly implemented `std::error::Error::source` for `Error` type
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
