// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

macro_rules! debug {
  ($span:ident, $fmt:expr $(,$args:expr)*) => {
    ::log::debug!(::std::concat!("{}: ", $fmt), $span $(,$args)*)
  };
}

macro_rules! error {
  ($span:ident, $fmt:expr $(,$args:expr)*) => {
    ::log::error!(::std::concat!("{}: ", $fmt), $span $(,$args)*)
  };
}

macro_rules! info {
  ($span:ident, $fmt:expr $(,$args:expr)*) => {
    ::log::info!(::std::concat!("{}: ", $fmt), $span $(,$args)*)
  };
}

macro_rules! trace {
  ($span:ident, $fmt:expr, $($args:expr)*) => {
    ::log::trace!(::std::concat!("{}: ", $fmt), $span, $($args)*)
  };
}
