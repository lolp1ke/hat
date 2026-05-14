use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops,
  sync::{Arc, atomic::AtomicUsize},
};

use parking_lot::RwLock;
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use crate::app::{App, AppContext, Context};

new_key_type! {
  pub struct EntityId;
}

#[derive(Debug)]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct Entity<T> {
  #[deref]
  #[deref_mut]
  any: AnyEntity,
  ty: PhantomData<T>,
}
impl<E> Entity<E> {
  pub fn new(entity_id: EntityId) -> Self
  where
    E: 'static,
  {
    Self {
      any: AnyEntity::new(entity_id, TypeId::of::<E>()),
      ty: PhantomData,
    }
  }
  pub fn read<'a>(&self, cx: &'a App) -> &'a E
  where
    E: 'static,
  {
    cx.entities.read(self)
  }
  pub fn update<C, F, R>(&self, cx: &mut C, f: F) -> R
  where
    C: AppContext,
    F: FnOnce(&mut E, &mut Context<E>) -> R,
    E: 'static,
  {
    cx.update_entity(self, f)
  }

  pub fn id(&self) -> EntityId {
    self.any.entity_id
  }
}
impl<T> Clone for Entity<T> {
  fn clone(&self) -> Self {
    Self {
      any: self.any.clone(),
      ty: self.ty,
    }
  }
}

#[derive(Debug)]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct Slot<T>(Entity<T>);

#[derive(Debug)]
pub struct AnyEntity {
  entity_id: EntityId,
  ty_id: TypeId,
}
impl AnyEntity {
  pub fn new(entity_id: EntityId, ty_id: TypeId) -> Self {
    Self { entity_id, ty_id }
  }
  pub fn downcast<E>(self) -> Option<Entity<E>>
  where
    E: 'static,
  {
    if TypeId::of::<E>() == self.ty_id {
      return Some(Entity {
        any: self,
        ty: PhantomData,
      });
    };

    None
  }
}
impl<E> From<Entity<E>> for AnyEntity
where
  E: 'static,
{
  fn from(value: Entity<E>) -> Self {
    Self::new(value.entity_id, TypeId::of::<E>())
  }
}
impl Clone for AnyEntity {
  fn clone(&self) -> Self {
    Self {
      entity_id: self.entity_id,
      ty_id: self.ty_id,
    }
  }
}

#[derive(Debug)]
#[derive(Default)]
pub struct EntityMap {
  pub(crate) entities: SecondaryMap<EntityId, Box<dyn Any>>,
  ref_counts: Arc<RwLock<EntityRefCounts>>,
}
impl EntityMap {
  pub fn reserve<E>(&self) -> Slot<E>
  where
    E: 'static,
  {
    let id = self.ref_counts.write().counts.insert(AtomicUsize::new(1));
    Slot(Entity::new(id))
  }
  pub fn read<E>(&self, handle: &Entity<E>) -> &E
  where
    E: 'static,
  {
    self
      .entities
      .get(handle.entity_id)
      .and_then(|entity| entity.downcast_ref::<E>())
      .unwrap()
  }
  pub fn insert<E>(&mut self, slot: Slot<E>, entity: E) -> Entity<E>
  where
    E: 'static,
  {
    let handle = slot.0;
    self.entities.insert(handle.entity_id, Box::new(entity));
    handle
  }

  pub(crate) fn lease<E>(&mut self, handle: &Entity<E>) -> Lease<E> {
    let entity = self
      .entities
      .remove(handle.entity_id)
      .unwrap_or_else(|| panic!("lease"));
    Lease::new(entity, handle.entity_id)
  }
  pub(crate) fn end_lease<E>(&mut self, lease: Lease<E>) {
    self.entities.insert(lease.entity_id, lease.entity);
  }
}

#[derive(Debug)]
#[derive(Default)]
pub struct EntityRefCounts {
  counts: SlotMap<EntityId, AtomicUsize>,
}
impl EntityRefCounts {
  pub fn new() -> Self {
    Self {
      counts: SlotMap::with_key(),
    }
  }
}

#[derive(Debug)]
pub(crate) struct Lease<E> {
  entity: Box<dyn Any>,
  pub(crate) entity_id: EntityId,
  entity_ty: PhantomData<E>,
}
impl<E> Lease<E> {
  pub const fn new(entity: Box<dyn Any>, entity_id: EntityId) -> Self {
    Self {
      entity,
      entity_id,
      entity_ty: PhantomData,
    }
  }
}
impl<E> ops::Deref for Lease<E>
where
  E: 'static,
{
  type Target = E;
  fn deref(&self) -> &Self::Target {
    self.entity.downcast_ref().unwrap()
  }
}
impl<E> ops::DerefMut for Lease<E>
where
  E: 'static,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.entity.downcast_mut().unwrap()
  }
}
