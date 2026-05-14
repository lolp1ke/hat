use std::{collections::HashMap, rc::Rc, sync::Arc};

use anyhow::Context as _;
use bitflags::bitflags;
use serde::Deserialize;
use smallvec::SmallVec;

use crate::app::{Action, App};

#[derive(Debug)]
#[derive(derive_more::Deref, derive_more::DerefMut)]
#[derive(Default)]
pub struct KeyBindings(pub(crate) Vec<KeyBinding>);
impl KeyBindings {
  pub fn add_bindings<T>(&mut self, bindings: T)
  where
    T: IntoIterator<Item = KeyBinding>,
  {
    for binding in bindings {
      self.0.push(binding);
    }
  }
}

#[derive(Debug)]
pub struct KeyBinding {
  pub action: Box<dyn Action>,
  pub keystrokes: SmallVec<[Keystroke; 2]>,
  pub key_context: Option<Rc<KeyContext>>,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub struct Keystroke {
  pub modifiers: Modifiers,
  pub key: Arc<str>,
  pub key_char: Option<Arc<str>>,
}
impl Keystroke {
  pub fn parse(source: &str) -> anyhow::Result<Self> {
    let mut modifiers = Modifiers::empty();
    let mut key = None;
    let mut key_char = None;

    let mut components = source.split('-').peekable();

    while let Some(component) = components.next() {
      if component.eq_ignore_ascii_case("ctrl") {
        modifiers |= Modifiers::CONTROL;
        continue;
      }
      if component.eq_ignore_ascii_case("alt") {
        modifiers |= Modifiers::ALT;
        continue;
      }
      if component.eq_ignore_ascii_case("shift") {
        modifiers |= Modifiers::SHIFT;
        continue;
      };
      if component.eq_ignore_ascii_case("meta")
        || component.eq_ignore_ascii_case("cmd")
        || component.eq_ignore_ascii_case("super")
        || component.eq_ignore_ascii_case("win")
      {
        modifiers |= Modifiers::META;
        continue;
      };

      let mut key_str = component.to_string();

      if let Some(next) = components.peek() {
        if next.is_empty() && source.ends_with('-') {
          key = Some(String::from("-"));
          break;
        } else if next.len() > 1 && next.starts_with('>') {
          key = Some(key_str.clone());
          components.next();
        } else {
          anyhow::bail!("Invalid keystroke: {}", source);
        }
        continue;
      }

      if component.len() == 1 && component.as_bytes()[0].is_ascii_uppercase() {
        // Convert to shift + lowercase char
        modifiers |= Modifiers::SHIFT;
        key_str.make_ascii_lowercase();
      } else {
        // convert ascii chars to lowercase so that named keys like "tab" and "enter"
        // are accepted case insensitively and stored how we expect so they are matched properly
        key_str.make_ascii_lowercase()
      }

      key = Some(key_str.clone());
      if modifiers.contains(Modifiers::SHIFT) {
        key_char = Some(key_str.to_uppercase().to_string());
      } else {
        key_char = Some(key_str.to_lowercase().to_string());
      };
    }

    // Allow for the user to specify a keystroke modifier as the key itself
    // This sets the `key` to the modifier, and disables the modifier
    // key = key.or_else(|| {
    //   use std::mem;
    //   // std::mem::take clears bool incase its true
    //   if mem::take(&mut modifiers.shift) {
    //     Some("shift".to_string())
    //   } else if mem::take(&mut modifiers.control) {
    //     Some("control".to_string())
    //   } else if mem::take(&mut modifiers.alt) {
    //     Some("alt".to_string())
    //   } else if mem::take(&mut modifiers.platform) {
    //     Some("platform".to_string())
    //   } else if mem::take(&mut modifiers.function) {
    //     Some("function".to_string())
    //   } else {
    //     None
    //   }
    // });

    let key = key
      .ok_or_else(|| anyhow::anyhow!("Invalid keystroke: {}", source))?
      .into();
    let key_char = key_char.and_then(|key_char| {
      if key_char.len() != 1 {
        None
      } else {
        Some(key_char.into())
      }
    });

    Ok(Self {
      modifiers,
      key,
      key_char,
    })
  }
}
bitflags! {
  #[derive(Debug)]
  #[derive(PartialEq)]
  pub struct Modifiers: u8 {
    const NONE = 1 << 0;
    const SHIFT = 1 << 1;
    const CONTROL = 1 << 2;
    const ALT = 1 << 3;
    const META = 1 << 4;
  }
}

#[derive(Debug)]
#[derive(Clone)]
pub enum KeyContext {
  Ident(Arc<str>),
  Eq(Arc<str>, Arc<str>),
  NEq(Arc<str>, Arc<str>),
  Not(Box<Self>),
  And(Box<Self>, Box<Self>),
  Or(Box<Self>, Box<Self>),
}
impl KeyContext {
  fn parse(source: &str) -> anyhow::Result<Self> {
    let source = remove_whitespace(source);
    let (context, rest) = Self::parse_expr(source, 0)?;
    if !rest.is_empty() {
      anyhow::bail!("Unexpected {}", rest);
    };
    Ok(context)
  }
  fn parse_expr(
    mut source: &str,
    min_precedence: u32,
  ) -> anyhow::Result<(Self, &str)> {
    type Op = fn(KeyContext, KeyContext) -> anyhow::Result<KeyContext>;

    let (mut lhs, rest) = Self::parse_primary(source)?;
    source = rest;

    'parse: loop {
      for (operator, precedence, constructor) in [
        ("&&", PRECEDENCE_AND, Self::new_and as Op),
        ("||", PRECEDENCE_OR, Self::new_or),
        ("==", PRECEDENCE_EQ, Self::new_eq),
        ("!=", PRECEDENCE_EQ, Self::new_neq),
      ] {
        if source.starts_with(operator) && precedence >= min_precedence {
          source = remove_whitespace(&source[operator.len()..]);
          let (rhs, rest) = Self::parse_expr(source, min_precedence)?;
          lhs = (constructor)(lhs, rhs)?;
          source = rest;
          continue 'parse;
        };
      }
      break 'parse;
    }

    Ok((lhs, source))
  }
  fn parse_primary(mut source: &str) -> anyhow::Result<(Self, &str)> {
    let next = source.chars().next().context("unexpected end")?;

    match next {
      '!' => {
        source = &source[1..];
        let (context, rest) = Self::parse_expr(source, PRECEDENCE_NOT)?;
        Ok((Self::Not(Box::new(context)), rest))
      }
      ch if is_ident_start_char(ch) => {
        let len = source.find(|ch| !is_ident_char(ch)).unwrap_or(source.len());
        let (ident, rest) = source.split_at(len);
        source = remove_whitespace(rest);
        Ok((Self::Ident(ident.into()), source))
      }
      _ => anyhow::bail!("Unexpected char: {}", next),
    }
  }

  fn new_and(self, other: Self) -> anyhow::Result<Self> {
    Ok(Self::And(Box::new(self), Box::new(other)))
  }
  fn new_or(self, other: Self) -> anyhow::Result<Self> {
    Ok(Self::Or(Box::new(self), Box::new(other)))
  }
  fn new_eq(self, other: Self) -> anyhow::Result<Self> {
    if let (Self::Ident(lhs), Self::Ident(rhs)) = (&self, &other) {
      Ok(Self::Eq(lhs.clone(), rhs.clone()))
    } else {
      anyhow::bail!("Ident expected, found: [{:?}, {:?}]", self, other);
    }
  }
  fn new_neq(self, other: Self) -> anyhow::Result<Self> {
    if let (Self::Ident(lhs), Self::Ident(rhs)) = (&self, &other) {
      Ok(Self::NEq(lhs.clone(), rhs.clone()))
    } else {
      anyhow::bail!("Ident expected, found: [{:?}, {:?}]", self, other);
    }
  }
}

const PRECEDENCE_OR: u32 = 2;
const PRECEDENCE_AND: u32 = 3;
const PRECEDENCE_EQ: u32 = 4;
const PRECEDENCE_NOT: u32 = 5;

#[inline]
fn remove_whitespace(src: &str) -> &str {
  let start = src
    .find(|ch: char| !ch.is_whitespace())
    .unwrap_or(src.len());
  &src[start..]
}
#[inline]
fn is_ident_char(ch: char) -> bool {
  ch.is_alphanumeric() || ch == '_'
}

#[inline]
fn is_ident_start_char(ch: char) -> bool {
  !ch.is_numeric() && is_ident_char(ch)
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct KeyBindingsFile {
  keybindings: Vec<KeyBindingsFileSection>,
}
impl KeyBindingsFile {
  pub fn parse(mut source: &str, cx: &mut App) -> anyhow::Result<KeyBindings> {
    source = source.trim();
    let keymap_file = toml::from_str::<Self>(source)?;

    let mut keybinds = Vec::new();

    for KeyBindingsFileSection { context, bindings } in
      keymap_file.keybindings.iter()
    {
      let context_predicate: Option<Rc<KeyContext>> = if context.is_empty() {
        None
      } else {
        Some(Rc::new(KeyContext::parse(context)?))
      };

      for (keystrokes, action) in bindings {
        let (action_name, _) = Self::parse_action(action)?;
        let keystrokes = keystrokes
          .split_whitespace()
          .flat_map(Keystroke::parse)
          .collect::<SmallVec<[Keystroke; 2]>>();

        let action = cx.actions.get_by_name(&format!(
          "{}{}",
          if context.is_empty() {
            String::with_capacity(0)
          } else {
            format!("{}::", context)
          },
          action_name
        ));
        keybinds.push(KeyBinding {
          action,
          key_context: context_predicate.clone(),
          keystrokes,
        });
      }
    }

    Ok(KeyBindings(keybinds))
  }

  fn parse_action(action: &toml::Value) -> anyhow::Result<(String, ())> {
    match action {
      toml::Value::String(s) => Ok((s.clone(), ())),
      _ => anyhow::bail!("`String` or 2 element `Array` expected"),
    }
  }
}
#[derive(Debug)]
#[derive(Deserialize)]
pub struct KeyBindingsFileSection {
  #[serde(default)]
  context: String,
  bindings: HashMap<String, toml::Value>,
}
