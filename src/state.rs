// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::{HashMap, HashSet},
  ops::Deref,
  path::Path,
  sync::Arc,
};

use dene::global::Global;
use qalam::PeerId;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct CurrentPersona(pub(crate) Arc<str>);
impl CurrentPersona {
  pub fn new(persona: Option<Arc<str>>) -> Self {
    if let Some(persona) = persona {
      Self(persona)
    } else {
      todo!("random name gen")
    }
  }
}
impl Global for CurrentPersona {}

#[derive(Clone)]
pub struct CurrentTopic(pub(crate) Arc<str>);
impl CurrentTopic {
  pub fn new(topic: Arc<str>) -> Self {
    Self(topic)
  }
}
impl Global for CurrentTopic {}
impl Deref for CurrentTopic {
  type Target = Arc<str>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[derive(Serialize, Deserialize)]
#[derive(Default)]
pub struct AddressBook {
  pub(crate) personas_by_peer_id: FxHashMap<Arc<str>, HashSet<Arc<str>>>,
  pub(crate) persona_by_peer_id: FxHashMap<Arc<str>, Arc<str>>,
}
impl AddressBook {
  pub fn try_load(path: &Path) -> anyhow::Result<Self> {
    let bytes = std::fs::read(path)?;
    Ok(toml::from_slice::<AddressBook>(&bytes)?)
  }

  pub fn insert(
    &mut self,
    peer: impl Into<Arc<str>>,
    persona: impl Into<Arc<str>>,
    update_default: bool,
  ) {
    let peer = peer.into();
    let persona = persona.into();

    if update_default {
      self
        .persona_by_peer_id
        .entry(peer.clone())
        .or_insert(persona.clone());
    };
    self
      .personas_by_peer_id
      .entry(peer)
      .or_default()
      .insert(persona);
  }
  pub fn get<K>(&self, key: K) -> Option<Arc<str>>
  where
    K: AsRef<str>,
  {
    self.persona_by_peer_id.get(key.as_ref()).cloned()
  }
}
impl Global for AddressBook {}
