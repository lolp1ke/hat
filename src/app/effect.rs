use std::any::{Any, TypeId};

use crate::app::EntityId;

#[derive(Debug)]
pub enum Effect {
  Notify {
    entity_id: EntityId,
  },
  Emit {
    emitter: EntityId,
    event_ty: TypeId,
    event: Box<dyn Any>,
  },
}
