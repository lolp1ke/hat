// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

#[derive(clap::Parser)]
#[command(
  version,
  about = "awesome temporary p2p chat with musical accompaniment"
)]
pub struct Args {
  #[clap()]
  pub persona: Option<Arc<str>>,
}
impl Args {
  pub fn new() -> Self {
    clap::Parser::parse()
  }
}
impl Default for Args {
  fn default() -> Self {
    Self::new()
  }
}
