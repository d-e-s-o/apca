Unreleased
----------
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
