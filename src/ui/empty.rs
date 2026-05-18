// SPDX-License-Identifier: Apache-2.0

use dene::{
  Context,
  ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders},
  },
  view::{Interactive, Render},
  window::Window,
};

pub struct Empty;
impl Render for Empty {
  fn render(
    &mut self,
    frame: &mut Frame,
    area: Rect,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    frame
      .render_widget(Block::new().borders(Borders::ALL).title("EMPTY"), area);
  }
}
impl Interactive for Empty {}
