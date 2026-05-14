use std::{any::TypeId, collections::HashMap, marker::PhantomData, rc::Rc};

use ratatui::layout::Rect;
use slotmap::new_key_type;

use crate::app::{
  Action, AnyView, App, AppContext, Keystroke, LayoutEngine, PaneId, PaneNode,
  clamp_area, draw,
};

type ActionListener = Rc<dyn Fn(&dyn Action, &mut Window, &mut App)>;

#[derive(derive_more::Debug)]
pub struct Window {
  handle: AnyWindowHandle,

  pub(crate) root: Option<PaneNode>,
  pub(crate) active_pane: Option<PaneId>,
  next_pane_id: u32,
  pub(crate) area: Rect,

  #[debug(skip)]
  action_listeners: HashMap<TypeId, Vec<ActionListener>>,

  layout_engine: LayoutEngine,
}
impl Window {
  pub fn new(handle: AnyWindowHandle, config: WindowConfig) -> Self {
    let WindowConfig { area, .. } = config;
    Self {
      handle,
      root: None,
      active_pane: Some(PaneId(0)),
      next_pane_id: 1,

      area,
      action_listeners: HashMap::default(),
      layout_engine: LayoutEngine::default(),
    }
  }

  pub fn next_pane_id(&mut self) -> PaneId {
    let id = PaneId(self.next_pane_id);
    self.next_pane_id += 1;
    id
  }

  pub fn render(&mut self, cx: &mut App) {
    let root_id = self.layout_engine.build(self.root.as_ref().unwrap());
    self.layout_engine.compute(
      root_id,
      self.area.width as f32,
      self.area.height as f32,
    );

    let views = {
      let mut pairs = Vec::new();
      self.root.as_ref().unwrap().visit_leaves(
        root_id,
        &self.layout_engine,
        self.area.x as f32,
        self.area.y as f32,
        &mut |pane, rect| pairs.push((pane.view.clone(), rect)),
      );
      pairs
    };

    draw(|frame| {
      for (view, area) in views {
        let area = clamp_area(area, self.area);
        if area.width == 0 || area.height == 0 {
          continue;
        };
        (view.render)(&view, frame, area, self, cx);
      }
    });
  }

  pub fn dispatch_action(&mut self, action: &dyn Action, cx: &mut App) {
    let action_ty = action.as_any().type_id();
    if let Some(global_listeners) =
      cx.global_action_listeners.remove(&action_ty)
    {
      for listener in global_listeners.iter() {
        (listener)(action, cx);
      }

      cx.global_action_listeners
        .insert(action_ty, global_listeners);
    };

    if let Some(listeners) = self.action_listeners.remove(&action_ty) {
      for listener in listeners.iter() {
        (listener)(action, self, cx);
      }

      self.action_listeners.insert(action_ty, listeners);
    };
  }
  pub fn dispatch_keystroke(&mut self, keystroke: Keystroke, cx: &mut App) {
    if let Some(node) = self.root.as_ref()
      && let Some(active_pane) = self.active_pane
      && let Some(pane) = node.find(active_pane)
    {
      let view = pane.view.clone();
      (view.on_keystroke)(&view, keystroke, self, cx);
    };
  }

  pub fn on_action<F, A>(&mut self, f: F)
  where
    F: 'static + Fn(&A, &mut Self, &mut App),
    A: Action,
  {
    self
      .action_listeners
      .entry(TypeId::of::<A>())
      .or_default()
      .push(Rc::new(move |action, window, cx| {
        let action = action.as_any().downcast_ref().expect("wrong action");
        f(action, window, cx);
      }));
  }
}

#[derive(Debug)]
pub struct WindowConfig {
  pub area: Rect,
}
impl Default for WindowConfig {
  fn default() -> Self {
    let (width, height) = crossterm::terminal::size().unwrap();

    Self {
      area: Rect {
        x: 0,
        y: 0,
        width,
        height,
      },
    }
  }
}

new_key_type! {
  pub struct WindowId;
}
#[derive(Debug)]
#[derive(Clone, Copy)]
pub struct AnyWindowHandle {
  pub(crate) window_id: WindowId,
  ty: TypeId,
}
impl AnyWindowHandle {
  fn new<W>(window_id: WindowId) -> Self
  where
    W: 'static,
  {
    Self {
      window_id,
      ty: TypeId::of::<W>(),
    }
  }

  pub fn update<C, F, R>(self, cx: &mut C, f: F) -> anyhow::Result<R>
  where
    C: AppContext,
    F: FnOnce(AnyView, &mut Window, &mut App) -> R,
  {
    cx.update_window(self, f)
  }
}
impl<W> From<WindowHandle<W>> for AnyWindowHandle {
  fn from(value: WindowHandle<W>) -> Self {
    value.handle
  }
}

#[derive(Debug)]
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct WindowHandle<W> {
  #[deref]
  #[deref_mut]
  handle: AnyWindowHandle,
  _marker: PhantomData<W>,
}
impl<W> WindowHandle<W> {
  pub fn new(window_id: WindowId) -> Self
  where
    W: 'static,
  {
    Self {
      handle: AnyWindowHandle::new::<W>(window_id),
      _marker: PhantomData,
    }
  }
}
impl<W> Copy for WindowHandle<W> {}
impl<W> Clone for WindowHandle<W> {
  fn clone(&self) -> Self {
    *self
  }
}
