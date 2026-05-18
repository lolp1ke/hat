// SPDX-License-Identifier: Apache-2.0

use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use dene::{
  Context, actions,
  event::EventEmitter,
  keybind::Keystroke,
  ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph},
  },
  view::{Interactive, Render},
  window::Window,
};

const CURSOR_BLINK_RATE: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub struct InputMessage {
  message: Vec<String>,

  cursor_pos: (u16, u16),
  cursor_visible: bool,
  cursor_last_blink: Instant,
}
impl InputMessage {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    window.on_action(cx.listener(Self::move_left));
    window.on_action(cx.listener(Self::move_right));
    window.on_action(cx.listener(Self::move_up));
    window.on_action(cx.listener(Self::move_down));
    window.on_action(cx.listener(Self::delete_at_cursor_pos));
    window.on_action(cx.listener(Self::insert_new_line_at_cursor_pos));
    window.on_action(cx.listener(Self::insert_space_at_cursor_pos));
    window.on_action(cx.listener(Self::send_message));

    Self {
      message: vec![String::default()],
      cursor_pos: (0, 0),
      cursor_visible: true,
      cursor_last_blink: Instant::now(),
    }
  }

  fn move_left(
    &mut self,
    _: &CursorLeft,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    self.cursor_pos = self.fix_cursor_pos();
    if self.cursor_pos.0 > 0 {
      self.cursor_pos.0 -= 1;
    };
  }
  fn move_right(
    &mut self,
    _: &CursorRight,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    self.cursor_pos.0 += 1;
  }
  fn move_up(&mut self, _: &CursorUp, _: &mut Window, _: &mut Context<Self>) {
    if self.cursor_pos.1 > 0 {
      self.cursor_pos.1 -= 1;
    };
  }
  fn move_down(
    &mut self,
    _: &CursorDown,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    self.cursor_pos.1 += 1;
    let (_, y) = self.fix_cursor_pos();
    self.cursor_pos.1 = y;
  }
  fn delete_at_cursor_pos(
    &mut self,
    _: &DeleteAtCursorPos,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    self.cursor_pos = self.fix_cursor_pos();
    let (x, y) = &mut self.cursor_pos;

    if *x > 0
      && let Some(line) = self.message.get_mut(*y as usize)
    {
      *x -= 1;
      line.remove(*x as usize);
    };
  }
  fn insert_new_line_at_cursor_pos(
    &mut self,
    _: &InsertNewLineAtCursorPos,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    self.cursor_pos = self.fix_cursor_pos();
    let (x, y) = self.cursor_pos;
    let x = x as usize;
    let y = y as usize;

    if y > self.message.len() {
      return;
    }

    if let Some(current_line) = self.message.get_mut(y) {
      let line = current_line.clone();

      let (start, end) = line.split_at(x);

      *current_line = start.to_string();

      self.message.insert(y + 1, end.to_string());

      self.cursor_pos.0 = 0;
      self.cursor_pos.1 += 1;
    };
  }
  fn insert_space_at_cursor_pos(
    &mut self,
    _: &InsertSpaceAtCursorPos,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    self.cursor_pos = self.fix_cursor_pos();
    if let Some(line) = self.message.get_mut(self.cursor_pos.1 as usize) {
      line.push(' ');
      self.cursor_pos.0 += 1;
    }
  }

  fn send_message(
    &mut self,
    _: &SendMessage,
    _: &mut Window,
    cx: &mut Context<Self>,
  ) {
    // early return if empty
    if self.message.len() <= 1
      && let Some(msg) = self.message.first()
      && msg.is_empty()
    {
      return;
    };

    let data = std::mem::take(&mut self.message);
    cx.emit(InputEvent::Submit {
      data: data.into_iter().map(|line| line.into()).collect(),
    });

    self.message.clear();
    self.message.push(String::new());
    self.cursor_pos = (0, 0);
  }

  fn tick_cursor(&mut self) {
    if self.cursor_last_blink.elapsed() >= CURSOR_BLINK_RATE {
      self.cursor_visible = !self.cursor_visible;
      self.cursor_last_blink = Instant::now();
    };
  }
  fn fix_cursor_pos(&self) -> (u16, u16) {
    let (mut x, mut y) = self.cursor_pos;

    if let Some(line) = self.message.get(y as usize) {
      x = x.min(line.len() as u16);
    };
    y = y.min(self.message.len().saturating_sub(1) as u16);
    (x, y)
  }
}
impl Render for InputMessage {
  fn render(
    &mut self,
    frame: &mut Frame,
    area: Rect,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    frame.render_widget(Clear, area);

    let input_block = Block::new().title("input").borders(Borders::ALL);
    let input_inner = input_block.inner(area);
    let input_message = Paragraph::new(Text::from(
      self
        .message
        .iter()
        .map(|l| Line::raw(l.clone()))
        .collect::<Vec<_>>(),
    ))
    .block(input_block);

    frame.render_widget(input_message, area);

    self.tick_cursor();
    if self.cursor_visible {
      let (x, y) = self.fix_cursor_pos();
      let x = input_inner.x + x;
      let y = input_inner.y + y;

      if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
        cell.set_symbol("█");
      };
    };
  }
}
impl Interactive for InputMessage {
  fn on_keystroke(
    &mut self,
    keystroke: Keystroke,
    _: &mut Window,
    _: &mut Context<Self>,
  ) {
    let Some(key_char) = keystroke.key_char else {
      return;
    };

    self.cursor_pos = self.fix_cursor_pos();
    let (x, y) = self.cursor_pos;
    let x = x as usize;
    let y = y as usize;

    let Some(line) = self.message.get_mut(y) else {
      return;
    };
    let right = line.split_off(x);

    for ch in key_char.chars() {
      line.push(ch);
      self.cursor_pos.0 += 1;
    }
    line.push_str(&right);
  }
}
impl EventEmitter<InputEvent> for InputMessage {}

#[derive(Debug)]
pub enum InputEvent {
  Submit { data: Arc<[Arc<str>]> },
}

actions! {
  "input",
  [
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    DeleteAtCursorPos,
    InsertNewLineAtCursorPos,
    InsertSpaceAtCursorPos,

    SendMessage,
  ]
}
