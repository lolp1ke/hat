use std::sync::Arc;

use libp2p::{gossipsub, mdns, swarm::NetworkBehaviour};

#[derive(derive_more::Debug)]
#[derive(NetworkBehaviour)]
pub struct HatNetwork {
  pub(crate) gossipsub: gossipsub::Behaviour,
  #[debug(skip)]
  pub(crate) mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
pub enum HatNetworkCommand {
  SendMessage { topic: Arc<str>, message: Vec<u8> },
}
