use std::sync::Arc;

use dene::global::Global;

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
