// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::api::v2::position::Position;
use crate::Str;


EndpointDef! {
  /// The representation of a GET request to the /v2/positions endpoint.
  pub Get(()),
  Ok => Vec<Position>, [
    /// The list of positions was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [ ]

  fn path(_input: &Self::Input) -> Str {
    "/v2/positions".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn list_positions() -> Result<(), Error> {
    // We can't do much here except check that the request is not
    // reporting any errors.
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let _ = client.issue::<Get>(()).await?;
    Ok(())
  }
}
