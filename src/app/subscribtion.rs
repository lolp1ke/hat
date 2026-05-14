use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use slotmap::{SlotMap, new_key_type};

new_key_type! {
  pub struct SubscribtionId;
}

pub struct SubscribtionSet<K, C> {
  subscribers: Rc<RefCell<BTreeMap<K, SlotMap<SubscribtionId, C>>>>,
}
impl<K, C> SubscribtionSet<K, C> {
  pub fn insert(&mut self, key: K, cb: C) -> SubscribtionId
  where
    K: Ord,
  {
    let mut lock = self.subscribers.borrow_mut();
    lock.entry(key).or_insert_with(SlotMap::with_key).insert(cb)
  }

  pub fn retain<F>(&mut self, key: K, mut f: F)
  where
    K: Ord,
    F: FnMut(&mut C) -> bool,
  {
    let mut lock = self.subscribers.borrow_mut();
    let Some(subscribers) = lock.get_mut(&key) else {
      return;
    };

    subscribers.retain(|_, c| f(c));
  }
}
impl<K, C> Default for SubscribtionSet<K, C> {
  fn default() -> Self {
    Self {
      subscribers: Rc::default(),
    }
  }
}
impl<K, C> Clone for SubscribtionSet<K, C> {
  fn clone(&self) -> Self {
    Self {
      subscribers: self.subscribers.clone(),
    }
  }
}
