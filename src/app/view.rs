use std::{io::Stdout, sync::OnceLock};

use parking_lot::RwLock;
use ratatui::{Frame, Terminal, layout::Rect, prelude::CrosstermBackend};

use crate::app::{
  AnyEntity, App, Entity, Interactive, Keystroke, Render, Window,
};

static TERM: OnceLock<RwLock<Terminal<CrosstermBackend<Stdout>>>> =
  OnceLock::new();

pub fn init_term() {
  // use std::io::Write;
  let term = ratatui::init();
  crossterm::execute!(
    std::io::stdout(),
    crossterm::event::PushKeyboardEnhancementFlags(
      crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
    )
  )
  .unwrap();

  TERM.set(RwLock::new(term)).unwrap();
}

pub fn draw<F, R>(f: F) -> R
where
  F: FnOnce(&mut Frame) -> R,
{
  let terminal = TERM.get().unwrap();
  let mut terminal = terminal.write();
  let mut result = None;
  terminal
    .draw(|frame| {
      result = Some(f(frame));
    })
    .unwrap();
  result.unwrap()
}

#[derive(Debug)]
#[derive(Clone)]
pub struct AnyView {
  entity: AnyEntity,
  pub render: fn(&Self, &mut Frame, Rect, &mut Window, &mut App),
  pub on_keystroke: fn(&Self, Keystroke, &mut Window, &mut App),
}
impl AnyView {
  pub fn downcast<E>(self) -> Option<Entity<E>>
  where
    E: 'static,
  {
    self.entity.downcast()
  }
}
impl<V> From<Entity<V>> for AnyView
where
  V: Render,
{
  fn from(value: Entity<V>) -> Self {
    Self {
      entity: value.into(),
      render: render::<V>,
      on_keystroke: on_keystroke::<V>,
    }
  }
}

fn render<V>(
  any_view: &AnyView,
  frame: &mut Frame,
  area: Rect,
  window: &mut Window,
  cx: &mut App,
) where
  V: 'static + Render,
{
  let view = any_view.clone().downcast::<V>().unwrap().clone();
  view.update(cx, |view, cx| {
    // draw(|frame| view.render(frame, area, window, cx))
    view.render(frame, area, window, cx);
  });
}
fn on_keystroke<V>(
  any_view: &AnyView,
  keystroke: Keystroke,
  window: &mut Window,
  cx: &mut App,
) where
  V: 'static + Interactive,
{
  let view = any_view.clone().downcast::<V>().unwrap();
  view.update(cx, |view, cx| view.on_keystroke(keystroke, window, cx));
}
