// Copyright (C) 2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::marker::PhantomData;
use std::pin::Pin;

use futures::task::Context;
use futures::task::Poll;
use futures::Sink;
use futures::SinkExt as _;
use futures::Stream;
use futures::StreamExt as _;

use websocket_util::tungstenite::Error as WebSocketError;


/// A wrapper around a stream that "unfolds" vectors of messages,
/// delivering them one by one.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
struct Unfold<S, T, E> {
  /// The wrapped stream & sink.
  inner: S,
  /// A vector of messages that we have received but not yet forwarded.
  messages: Vec<T>,
  /// Phantom data to make sure that we "use" `E`.
  _phantom: PhantomData<E>,
}

impl<S, T, E> Unfold<S, T, E> {
  /// Create a new `Unfold` object wrapping the provided stream.
  pub(crate) fn new(inner: S) -> Self {
    Self {
      inner,
      messages: Vec::new(),
      _phantom: PhantomData,
    }
  }
}

impl<S, T, E> Stream for Unfold<S, T, E>
where
  S: Stream<Item = Result<Result<Vec<T>, E>, WebSocketError>> + Unpin,
  T: Unpin,
  E: Unpin,
{
  type Item = Result<Result<T, E>, WebSocketError>;

  fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    loop {
      if !self.messages.is_empty() {
        // In order to preserve ordering we pop from the front. That is
        // vastly ineffective for large vectors, though, because
        // subsequent elements all will have to be copied.
        let message = self.messages.remove(0);
        break Poll::Ready(Some(Ok(Ok(message))))
      } else {
        match self.inner.poll_next_unpin(ctx) {
          Poll::Pending => {
            // No new data is available yet. There is nothing to do for us
            // except bubble up this result.
            break Poll::Pending
          },
          Poll::Ready(None) => {
            // The stream is exhausted. Bubble up the result and be done.
            break Poll::Ready(None)
          },
          Poll::Ready(Some(Err(err))) => break Poll::Ready(Some(Err(err))),
          Poll::Ready(Some(Ok(Err(err)))) => break Poll::Ready(Some(Ok(Err(err)))),
          Poll::Ready(Some(Ok(Ok(messages)))) => {
            self.messages = messages;
            // Continue above by popping from `messages`.
          },
        }
      }
    }
  }
}

impl<S, T, E, U> Sink<U> for Unfold<S, T, E>
where
  S: Sink<U, Error = WebSocketError> + Unpin,
  T: Unpin,
  E: Unpin,
{
  type Error = WebSocketError;

  fn poll_ready(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    self.inner.poll_ready_unpin(ctx)
  }

  fn start_send(mut self: Pin<&mut Self>, message: U) -> Result<(), Self::Error> {
    self.inner.start_send_unpin(message)
  }

  fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    self.inner.poll_flush_unpin(ctx)
  }

  fn poll_close(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    self.inner.poll_close_unpin(ctx)
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use futures::stream::iter;
  use futures::TryStreamExt as _;

  use test_log::test;


  /// Check that we can unfold a stream of vectors of messages.
  #[test(tokio::test)]
  #[allow(unused_qualifications)]
  async fn unfolding() {
    let it = iter([vec![1], vec![2, 3, 4], vec![], vec![5, 6]])
      .map(Result::<_, ()>::Ok)
      .map(Ok);

    let stream = Unfold::new(it);
    let result = stream.try_collect::<Vec<_>>().await.unwrap();
    let expected = (1..=6).map(Ok).collect::<Vec<_>>();
    assert_eq!(result, expected);
  }
}
