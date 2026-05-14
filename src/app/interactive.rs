use crate::app::{Context, Keystroke, Window};

#[expect(unused_variables, reason = "default noop implementation")]
pub trait Interactive: 'static + Sized {
  fn on_keystroke(
    &mut self,
    keystroke: Keystroke,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
  }
}
