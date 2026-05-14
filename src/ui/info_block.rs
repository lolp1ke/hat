use std::sync::Arc;

use ratatui::{
  Frame,
  layout::Rect,
  text::{Line, Text},
  widgets::{Block, Borders, Paragraph},
};
use time_format::strftime_local;

use crate::{
  app::{Context, Interactive, Render, Window},
  state::CurrentPersona,
};

#[derive(Debug)]
pub struct InfoBlock {
  persona: Arc<str>,
}
impl InfoBlock {
  pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
    let persona = cx.global::<CurrentPersona>().clone();

    Self { persona: persona.0 }
  }
}
impl Render for InfoBlock {
  fn render(
    &mut self,
    frame: &mut Frame,
    area: Rect,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    let info_block = Block::new().borders(Borders::ALL).title("info");
    let info = Paragraph::new(Text::from(vec![
      Line::raw(format!("persona: {}", self.persona)),
      Line::raw(format!(
        "time: {}",
        strftime_local("%H:%M:%S %p", time_format::now().unwrap()).unwrap()
      )),
    ]))
    .block(info_block);
    frame.render_widget(info, area);
  }
}
impl Interactive for InfoBlock {}
