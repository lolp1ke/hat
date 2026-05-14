use ratatui::{
  Frame,
  layout::Rect,
  widgets::{Block, Borders},
};

use crate::app::{Context, Interactive, Render, Window};

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
