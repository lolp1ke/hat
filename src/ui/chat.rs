use std::sync::Arc;

use dene::{
  Context,
  event::EventEmitter,
  ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
  },
  view::{Interactive, Render},
  window::Window,
};

use crate::ui::InputEvent;

#[derive(Debug)]
pub struct Chat {
  pub(crate) messages: Vec<(Arc<str>, Arc<[Arc<str>]>)>,
}
impl Chat {
  pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
    Self {
      messages: Vec::default(),
    }
  }
}
impl Render for Chat {
  fn render(
    &mut self,
    frame: &mut Frame,
    area: Rect,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    let chat_block = Block::new().borders(Borders::ALL).title("chat");
    let inner = chat_block.inner(area);

    let lines = self
      .messages
      .iter()
      .flat_map(|(sender, content)| {
        let sender_line = Line::from(Span::styled(
          &**sender,
          Style::new().bold().fg(Color::Cyan),
        ));

        let msg_lines =
          content.iter().map(|line| Line::from(Span::raw(&**line)));

        std::iter::once(sender_line).chain(msg_lines)
      })
      .collect::<Vec<_>>();

    let scroll_offset = (lines.len() as u16).saturating_sub(inner.height);

    let chat = Paragraph::new(lines)
      .block(chat_block)
      .wrap(Wrap { trim: false })
      .scroll((scroll_offset, 0));

    frame.render_widget(chat, area);
  }
}
impl Interactive for Chat {}
impl EventEmitter<InputEvent> for Chat {}
