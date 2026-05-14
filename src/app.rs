pub mod action;
mod effect;
mod entity;
mod event;
mod executor;
mod global;
mod interactive;
mod keybindings;
mod layout;
mod pane;
mod render;
mod subscribtion;
mod view;
mod window;

pub use action::*;
pub use effect::*;
pub use entity::*;
pub use event::*;
pub use executor::*;
pub use global::*;
pub use interactive::*;
pub use keybindings::*;
pub use layout::*;
pub use pane::*;
use ratatui::layout::Size;
pub use render::*;
pub use subscribtion::*;
pub use view::*;
pub use window::*;

use std::{
  any::{Any, TypeId},
  cell::RefCell,
  collections::{HashMap, HashSet, VecDeque},
  fmt::Debug,
  rc::{self, Rc, Weak},
  sync::atomic::{self, AtomicBool},
};

use anyhow::Context as _;
use crossterm::event::{self as term_event, EventStream, KeyModifiers};
use futures_util::{FutureExt, StreamExt as _};
use slotmap::SlotMap;
use tokio::{self, sync::mpsc::UnboundedReceiver};

use crate::app;

type GlobalActionListener = Rc<dyn 'static + Fn(&dyn Action, &mut App)>;

type EventListener = Box<dyn 'static + FnMut(&dyn Any, &mut App) -> bool>;

#[derive(derive_more::Debug)]
pub struct App {
  this: rc::Weak<RefCell<Self>>,
  quitting: AtomicBool,

  foreground_executor: ForegroundExecutor,
  background_executor: BackgroundExecutor,
  globals_by_type: HashMap<TypeId, Box<dyn Any>>,
  actions: Rc<ActionRegistry>,
  keybinds: Rc<RefCell<KeyBindings>>,

  #[debug(skip)]
  global_action_listeners: HashMap<TypeId, Vec<GlobalActionListener>>,

  #[debug(skip)]
  event_listeners: SubscribtionSet<EntityId, (TypeId, EventListener)>,

  windows: SlotMap<WindowId, Option<Box<Window>>>,
  active_window: Option<AnyWindowHandle>,

  pub(crate) entities: EntityMap,
  pending_updates: usize,
  pending_notifications: HashSet<EntityId>,
  pending_effects: VecDeque<Effect>,
  flushing_effects: bool,
}
impl App {
  pub fn new(
    foreground_executor: ForegroundExecutor,
    background_executor: BackgroundExecutor,
  ) -> Rc<RefCell<Self>> {
    Rc::new_cyclic(|this| {
      RefCell::new(Self {
        this: this.clone(),
        quitting: AtomicBool::new(false),
        foreground_executor,
        background_executor,
        globals_by_type: HashMap::default(),
        actions: Rc::new(ActionRegistry::new()),
        keybinds: Rc::default(),
        global_action_listeners: HashMap::default(),
        event_listeners: SubscribtionSet::default(),
        windows: SlotMap::default(),
        active_window: None,
        entities: EntityMap::default(),
        pending_updates: 0,
        pending_notifications: HashSet::default(),
        pending_effects: VecDeque::default(),
        flushing_effects: false,
      })
    })
  }

  pub fn load_keybinds(&mut self, keybindings: KeyBindings) {
    self.bind_keys(keybindings.0);
  }

  fn apply_notify(&mut self, entity_id: EntityId) {
    self.pending_notifications.remove(&entity_id);

    println!("apply observer handle");
  }
  fn apply_emit(
    &mut self,
    emitter: EntityId,
    event_ty: TypeId,
    event: &dyn Any,
  ) {
    self
      .event_listeners
      .clone()
      .retain(emitter, |(_event_ty, cb)| {
        if *_event_ty == event_ty {
          cb(event, self)
        } else {
          true
        }
      });
  }
  pub fn notify(&mut self, entity_id: EntityId) {
    if self.pending_notifications.insert(entity_id) {
      self.pending_effects.push_back(Effect::Notify { entity_id });
    };
  }
  pub fn update<F, R>(&mut self, f: F) -> R
  where
    F: FnOnce(&mut Self) -> R,
  {
    self.pending_updates += 1;
    let result = f(self);
    self.finish_update();
    result
  }
  fn finish_update(&mut self) {
    if !self.flushing_effects && self.pending_updates == 1 {
      self.flushing_effects = true;
      self.flush_effects();
      self.flushing_effects = false;
    };
    self.pending_updates -= 1;
  }
  fn flush_effects(&mut self) {
    while let Some(effect) = self.pending_effects.pop_front() {
      match effect {
        Effect::Notify { entity_id } => {
          self.apply_notify(entity_id);
        }
        Effect::Emit {
          emitter,
          event_ty,
          event,
        } => {
          self.apply_emit(emitter, event_ty, &*event);
        }
      };
    }
  }

  fn handle_key_event(
    &mut self,
    key: term_event::KeyEvent,
  ) -> anyhow::Result<()> {
    let mut keystroke = String::new();

    if matches!(key.modifiers, KeyModifiers::SHIFT) {
      keystroke.push_str("shift-");
    };
    if matches!(key.modifiers, KeyModifiers::CONTROL) {
      keystroke.push_str("ctrl-");
    };
    if matches!(key.modifiers, KeyModifiers::ALT) {
      keystroke.push_str("alt-");
    };
    if matches!(
      key.modifiers,
      KeyModifiers::META | KeyModifiers::SUPER | KeyModifiers::HYPER
    ) {
      keystroke.push_str("meta-");
    };
    keystroke.push_str(&key.code.to_string());

    if let Ok(keystroke) = Keystroke::parse(&keystroke) {
      let keybinds = self.keybinds.clone();

      for keybind in keybinds.borrow().iter() {
        if let Some(keystroke1) = keybind.keystrokes.first()
          && *keystroke1 == keystroke
        {
          self.dispatch_action(&*keybind.action);
        };
      }
      // TODO: save for second keybind if no action
      //       e.g: cmd+k cmd+l

      if let Some(active_window) = self.active_window {
        active_window.update(self, |_, window, cx| {
          window.dispatch_keystroke(keystroke, cx);
        })?;
      };
    };

    Ok(())
  }
  fn handle_event(&mut self, event: term_event::Event) -> anyhow::Result<()> {
    match event {
      term_event::Event::Key(key) => {
        self.handle_key_event(key)?;
      }
      term_event::Event::Resize(width, height) => {
        for (_, window) in self.windows.iter_mut() {
          if let Some(window) = window {
            window.area = window.area.resize(Size::new(width, height));
          };
        }
      }

      _ => {}
    };

    Ok(())
  }
  pub fn run<F, R>(
    app: Rc<RefCell<Self>>,
    mut foreground_rx: UnboundedReceiver<FgTask>,
    f: F,
  ) -> impl Future<Output = anyhow::Result<()>>
  where
    F: FnOnce(&mut Self) -> R,
  {
    app::init_term();
    f(&mut app.borrow_mut());

    app.borrow_mut().on_action(move |_: &action::Quit, cx| {
      cx.quitting.store(true, atomic::Ordering::Relaxed);
    });

    async move {
      let mut event_reader = EventStream::new();
      let mut tick =
        tokio::time::interval(tokio::time::Duration::from_secs_f64(1.0 / 24.0));

      while !app.borrow().quitting.load(atomic::Ordering::Relaxed) {
        tokio::select! {
          Some(Ok(event)) = event_reader.next() => {
            app.borrow_mut().handle_event(event)?;
          }
          Some(runnable) = foreground_rx.recv() => {
            runnable();
          }
          _ = tick.tick() => {
            let mut windows = std::mem::take(&mut app.borrow_mut().windows);
            for window in windows.iter_mut().flat_map(|(_, window)| window) {
              window.render(&mut app.borrow_mut());
            }
            app.borrow_mut().windows = windows;
          }
        }
      }

      app.borrow_mut().shutdown();
      Ok(())
    }
  }
  fn shutdown(&mut self) {
    ratatui::restore();
  }

  pub fn open_window<F, V>(
    &mut self,
    pane_config: WindowConfig,
    f: F,
  ) -> WindowHandle<V>
  where
    F: 'static + FnOnce(&mut Window, &mut Self) -> Entity<V>,
    V: 'static + Render,
  {
    self.update(|cx| {
      let window_id = cx.windows.insert(None);
      let handle = WindowHandle::new(window_id);
      let mut window = Window::new(handle.into(), pane_config);

      let root_view = f(&mut window, cx);
      if window.root.is_none() {
        let pane_id = window.next_pane_id();
        window.root.replace(PaneNode::Leaf(Pane {
          id: pane_id,
          view: root_view.into(),
        }));
        window.active_pane.replace(pane_id);
      };
      window.render(cx);

      cx.windows
        .get_mut(window_id)
        .unwrap()
        .replace(Box::new(window));
      cx.active_window = Some(*handle);
      handle
    })
  }
  fn update_window_id<F, R>(&mut self, id: WindowId, f: F) -> anyhow::Result<R>
  where
    F: FnOnce(AnyView, &mut Window, &mut App) -> R,
  {
    self
      .update(|cx| {
        let mut window = cx.windows.get_mut(id)?.take()?;

        let view = window
          .root
          .as_ref()?
          .find(window.active_pane?)
          .map(|p| p.view.clone())?;

        let result = f(view, &mut window, cx);
        cx.windows.get_mut(id)?.replace(window);
        Some(result)
      })
      .context("window not found")
  }

  fn bind_keys<I>(&mut self, bindings: I)
  where
    I: IntoIterator<Item = KeyBinding>,
  {
    let mut lock = self.keybinds.borrow_mut();
    lock.add_bindings(bindings);
  }
  fn on_action<F, A>(&mut self, listener: F)
  where
    F: 'static + Fn(&A, &mut Self),
    A: Action,
  {
    self
      .global_action_listeners
      .entry(TypeId::of::<A>())
      .or_default()
      .push(Rc::new(move |action, cx| {
        let action = action.as_any().downcast_ref().unwrap();
        (listener)(action, cx);
      }));
  }
  fn dispatch_action(&mut self, action: &dyn Action) {
    if let Some(active_window) = self.active_window {
      active_window
        .update(self, |_, window, cx| {
          window.dispatch_action(action, cx);
        })
        .unwrap();
    } else {
      self.dispatch_global_action_listener(action);
    };
  }

  fn dispatch_global_action_listener(&mut self, action: &dyn Action) {
    let action_ty = action.as_any().type_id();
    if let Some(listeners) = self.global_action_listeners.remove(&action_ty) {
      for listener in listeners.iter() {
        (listener)(action, self)
      }

      self.global_action_listeners.insert(action_ty, listeners);
    };
  }

  pub fn subscribe<E, F, Event>(&mut self, entity: Entity<E>, mut on_event: F)
  where
    E: 'static + EventEmitter<Event>,
    F: 'static + FnMut(Entity<E>, &Event, &mut App),
    Event: 'static,
  {
    self.event_listeners.insert(
      entity.id(),
      (
        TypeId::of::<Event>(),
        Box::new(move |event, cx| {
          let event = event.downcast_ref().expect("wrong event type");
          on_event(entity.clone(), event, cx);
          true
        }),
      ),
    );
  }

  pub fn spawn<AsyncFn, R>(&self, f: AsyncFn) -> Task<R>
  where
    AsyncFn: 'static + AsyncFnOnce(&mut AsyncApp) -> R,
    R: 'static,
  {
    let mut cx = self.to_async();
    self
      .foreground_executor
      .spawn(async move { f(&mut cx).await }.boxed_local())
  }
  pub fn spawn_on_background<Fut, R>(&self, f: Fut) -> Task<R>
  where
    Fut: 'static + Future<Output = R> + Send,
    Fut::Output: Send,
    R: 'static,
  {
    self.background_executor.spawn(f)
  }

  pub fn to_async(&self) -> AsyncApp {
    AsyncApp {
      app: self.this.clone(),
      foreground_executor: self.foreground_executor.clone(),
      background_executor: self.background_executor.clone(),
    }
  }

  pub fn global<G>(&self) -> &G
  where
    G: Global,
  {
    self.try_global().unwrap()
  }
  pub fn try_global<G>(&self) -> Option<&G>
  where
    G: Global,
  {
    self
      .globals_by_type
      .get(&TypeId::of::<G>())
      .and_then(|any| any.downcast_ref())
  }
  pub fn global_mut<G>(&mut self) -> &mut G
  where
    G: Global,
  {
    self.try_global_mut().unwrap()
  }
  pub fn try_global_mut<G>(&mut self) -> Option<&mut G>
  where
    G: Global,
  {
    self
      .globals_by_type
      .get_mut(&TypeId::of::<G>())
      .and_then(|any| any.downcast_mut())
  }
  pub fn set_global<G>(&mut self, global: G)
  where
    G: Global,
  {
    self
      .globals_by_type
      .insert(TypeId::of::<G>(), Box::new(global));
  }
}
impl AppContext for App {
  fn new_entity<F, E>(&mut self, f: F) -> Entity<E>
  where
    F: FnOnce(&mut Context<E>) -> E,
    E: 'static,
  {
    self.update(|app| {
      let slot = app.entities.reserve();
      let handle = slot.clone();
      let entity = f(&mut Context::new(app, handle.clone()));

      app.entities.insert(slot, entity);
      handle
    })
  }
  fn read_entity<E, F, R>(&self, handle: &Entity<E>, f: F) -> R
  where
    E: 'static,
    F: FnOnce(&E, &App) -> R,
  {
    let entity = self.entities.read(handle);
    f(entity, self)
  }
  fn update_entity<E, F, R>(&mut self, handle: &Entity<E>, f: F) -> R
  where
    F: FnOnce(&mut E, &mut Context<E>) -> R,
    E: 'static,
  {
    self.update(|app| {
      let mut lease = app.entities.lease(handle);
      let result = f(&mut lease, &mut Context::new(app, handle.clone()));
      app.entities.end_lease(lease);
      result
    })
  }

  fn update_window<F, R>(
    &mut self,
    handle: AnyWindowHandle,
    f: F,
  ) -> anyhow::Result<R>
  where
    F: FnOnce(AnyView, &mut Window, &mut App) -> R,
  {
    self.update_window_id(handle.window_id, f)
  }
}
impl Drop for App {
  fn drop(&mut self) {
    self.shutdown();
  }
}

#[derive(Debug)]
#[derive(Clone)]
pub struct AsyncApp {
  app: Weak<RefCell<App>>,
  foreground_executor: ForegroundExecutor,
  background_executor: BackgroundExecutor,
}
impl AsyncApp {
  pub fn app(&self) -> Rc<RefCell<App>> {
    self.app.upgrade().expect("App already dropped")
  }

  pub fn spawn<AsyncFn, R>(&self, f: AsyncFn) -> Task<R>
  where
    AsyncFn: 'static + AsyncFnOnce(&mut AsyncApp) -> R,
    R: 'static,
  {
    let mut cx = self.clone();
    self
      .foreground_executor
      .spawn(async move { f(&mut cx).await }.boxed_local())
  }
  pub fn spawn_on_background<Fut, R>(&self, future: Fut) -> Task<R>
  where
    Fut: 'static + Future<Output = R> + Send,
    Fut::Output: Send,
    R: 'static,
  {
    self.background_executor.spawn(future)
  }
}

#[derive(Debug)]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct Context<'a, E> {
  #[deref]
  #[deref_mut]
  app: &'a mut App,
  entity: Entity<E>,
}
impl<'a, E> Context<'a, E> {
  pub fn new(app: &'a mut App, entity: Entity<E>) -> Self {
    Self { app, entity }
  }

  pub fn entity(&self) -> Entity<E> {
    self.entity.clone()
  }

  pub fn subscribe<E2, F, Event>(&mut self, entity: Entity<E2>, on_event: F)
  where
    E2: 'static + EventEmitter<Event>,
    F: 'static + FnMut(Entity<E2>, &Event, &mut App),
    Event: 'static,
  {
    self.app.subscribe(entity, on_event);
  }
  pub fn subscribe_self<F, Event>(&mut self, on_event: F)
  where
    E: 'static + EventEmitter<Event>,
    F: 'static + FnMut(Entity<E>, &Event, &mut App),
    Event: 'static,
  {
    self.app.subscribe(self.entity(), on_event);
  }

  pub fn emit<Event>(&mut self, event: Event)
  where
    Event: 'static,
  {
    self.app.pending_effects.push_back(Effect::Emit {
      emitter: self.entity.id(),
      event_ty: TypeId::of::<Event>(),
      event: Box::new(event),
    });
  }

  pub fn listener<F, E2>(
    &self,
    f: F,
  ) -> impl 'static + Fn(&E2, &mut Window, &mut App)
  where
    F: 'static + Fn(&mut E, &E2, &mut Window, &mut Context<E>),
    E: 'static,
    E2: ?Sized,
  {
    let view = self.entity();
    move |e: &E2, window: &mut Window, cx: &mut App| {
      view.update(cx, |view, cx| f(view, e, window, cx))
    }
  }
}

pub trait AppContext {
  fn new_entity<F, E>(&mut self, f: F) -> Entity<E>
  where
    F: FnOnce(&mut Context<E>) -> E,
    E: 'static;

  fn read_entity<E, F, R>(&self, handle: &Entity<E>, f: F) -> R
  where
    E: 'static,
    F: FnOnce(&E, &App) -> R;

  fn update_entity<E, F, R>(&mut self, handle: &Entity<E>, f: F) -> R
  where
    F: FnOnce(&mut E, &mut Context<E>) -> R,
    E: 'static;

  fn update_window<F, R>(
    &mut self,
    window: AnyWindowHandle,
    f: F,
  ) -> anyhow::Result<R>
  where
    F: FnOnce(AnyView, &mut Window, &mut App) -> R;
}
