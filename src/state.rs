use std::sync::Arc;

use crate::app::Global;

#[derive(Clone)]
#[derive(derive_more::Deref)]
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
#[derive(derive_more::Deref)]
pub struct CurrentTopic(pub(crate) Arc<str>);
impl CurrentTopic {
  pub fn new(topic: Arc<str>) -> Self {
    Self(topic)
  }
}
impl Global for CurrentTopic {}
