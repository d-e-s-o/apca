Unreleased
----------
- Migrated streaming functionality from `websocket` to
  `async-tungstenite`
- Dropped dependency on `futures` `0.1`


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
