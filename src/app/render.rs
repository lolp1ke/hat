use ratatui::{Frame, layout::Rect};

use crate::app::{Context, Interactive, Window};

#[expect(unused_variables, reason = "default noop implementation")]
pub trait Render: 'static + Sized + Interactive {
  fn render(
    &mut self,
    frame: &mut Frame,
    area: Rect,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
  }
}
