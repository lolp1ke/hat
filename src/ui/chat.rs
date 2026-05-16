use std::sync::Arc;

use dene::{
  Context,
  event::EventEmitter,
  ratatui::{
    Frame,
    layout::Rect,
    text::Text,
    widgets::{Block, Borders, Paragraph},
  },
  view::{Interactive, Render},
  window::Window,
};

use crate::ui::InputEvent;

#[derive(Debug)]
pub struct Chat {
  pub(crate) messages: Vec<Arc<[Arc<str>]>>,
}
impl Chat {
  pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
    // let e = cx.entity();
    // cx.subscribe(e, |chat, event, cx| {
    //   chat.update(cx, |chat, _| {
    //     match event {
    //       ChatEvent::NewMessage { topic: _, message } => {
    //         chat.messages.push(message.clone());
    //       }
    //     };
    //   });
    // });

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
    let message_lines = self
      .messages
      .clone()
      .into_iter()
      .map(|lines| lines.join("\n"))
      .collect::<Vec<_>>();
    let chat = Paragraph::new(Text::from_iter(message_lines)).block(chat_block);
    frame.render_widget(chat, area);
  }
}
impl Interactive for Chat {}
// impl EventEmitter<ChatEvent> for Chat {}
impl EventEmitter<InputEvent> for Chat {}

// #[derive(Debug)]
// pub enum ChatEvent {
//   NewMessage {
// topic: TopicHash,
//     message: Arc<[Arc<str>]>,
//   },
// }

// #[derive(Debug)]
// pub struct Message {
//   id: Uuid,
//   content: Arc<[Arc<str>]>,
//   from: PeerId,
//   timestamp: Instant,
// }
