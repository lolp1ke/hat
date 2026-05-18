// SPDX-License-Identifier: Apache-2.0

use std::{
  collections::{BTreeMap, HashMap},
  sync::Arc,
};

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
use qalam::{
  PeerId,
  room::{ChatMessage, RoomId},
};

use crate::{
  state::{AddressBook, CurrentTopic},
  ui::InputEvent,
};

#[derive(Debug)]
pub struct Chat {
  topic: Arc<str>,
  pub(crate) messages: BTreeMap<RoomId, Vec<ChatMessage>>,
  local_peer_id: PeerId,
}
impl Chat {
  pub fn new(
    local_peer_id: PeerId,
    _: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    Self {
      topic: cx.global::<CurrentTopic>().0.clone(),
      messages: BTreeMap::default(),
      local_peer_id,
    }
  }
  pub fn set_topic(&mut self, topic: Arc<str>) {
    self.topic = topic;
  }
}
impl Render for Chat {
  fn render(
    &mut self,
    frame: &mut Frame,
    area: Rect,
    _: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let chat_block = Block::new()
      .borders(Borders::ALL)
      .title(Line::raw(format!("chat/{}", &*self.topic)).left_aligned());

    let inner = chat_block.inner(area);
    let topic = cx.global::<CurrentTopic>();
    let room = RoomId::room(topic);

    let _dummy = &Vec::new();
    let lines = self
      .messages
      .get(&room)
      .unwrap_or(_dummy)
      .iter()
      .flat_map(|msg| {
        let is_me = msg.from == self.local_peer_id;
        let from = msg.from.to_string();

        let address_book = cx.global::<AddressBook>();
        let persona_ident = address_book.get(msg.from.to_string());
        let peer_id_suffix = &from[from.len().saturating_sub(4)..];

        let sender_style = if is_me {
          Style::default().bold().fg(Color::Green)
        } else {
          Style::default().bold().fg(Color::Cyan)
        };
        let sender_line = Line::from(Span::styled(
          format!(
            "{}#{}: {}",
            persona_ident.unwrap_or("?".into()),
            peer_id_suffix,
            time_format::strftime_local(
              "%I:%M %p",
              (msg.ts.as_millis() / 1000) as i64
            )
            .unwrap_or_default(),
          ),
          sender_style,
        ));
        let msg_lines = msg
          .content
          .iter()
          .map(|line| Line::from(Span::raw(&**line)));

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
