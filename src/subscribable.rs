// Copyright (C) 2021-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use async_trait::async_trait;

use crate::Error;


/// A trait representing "something" that users can subscribe to to
/// receive updates through a stream.
#[async_trait]
pub trait Subscribable {
  /// Input required to establish a connection.
  type Input;
  /// The type of the subscription being provided.
  type Subscription;
  /// The output stream.
  type Stream;

  /// Establish a connection to receive updates and return a stream
  /// along with a subscription to control the stream, if applicable.
  async fn connect(input: &Self::Input) -> Result<(Self::Stream, Self::Subscription), Error>;
}
