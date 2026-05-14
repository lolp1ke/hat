use ratatui::layout::Rect;
use taffy::NodeId;

use crate::app::{AnyView, LayoutEngine};

#[derive(Debug)]
#[derive(Clone, Copy)]
#[derive(PartialEq)]
pub struct PaneId(pub(crate) u32);

#[derive(Debug)]
#[derive(Clone)]
pub struct Pane {
  pub(crate) id: PaneId,
  pub(crate) view: AnyView,
}

#[derive(Debug)]
#[derive(Clone)]
pub enum PaneNode {
  Leaf(Pane),
  Split {
    direction: Direction,
    children: Vec<PaneNode>,
    weights: Vec<f32>,
  },
}
impl PaneNode {
  pub fn visit_leaves<F>(
    &self,
    node_id: NodeId,
    engine: &LayoutEngine,
    offset_x: f32,
    offset_y: f32,
    f: &mut F,
  ) where
    F: FnMut(&Pane, Rect),
  {
    let layout = engine.layout(node_id);
    let abs_x = offset_x + layout.location.x;
    let abs_y = offset_y + layout.location.y;
    let width = layout.size.width.ceil() as u16;
    let height = layout.size.height.ceil() as u16;

    match self {
      Self::Leaf(pane) => {
        f(
          pane,
          Rect {
            x: abs_x as u16,
            y: abs_y as u16,
            width,
            height,
          },
        );
      }
      Self::Split { children, .. } => {
        let child_ids = engine.children(node_id);
        for (child, &child_id) in children.iter().zip(child_ids.iter()) {
          child.visit_leaves(child_id, engine, abs_x, abs_y, f);
        }
      }
    };
  }

  pub fn find(&self, id: PaneId) -> Option<&Pane> {
    match self {
      PaneNode::Leaf(pane) if pane.id == id => Some(pane),
      PaneNode::Leaf(..) => None,
      PaneNode::Split { children, .. } => {
        children.iter().find_map(|child| child.find(id))
      }
    }
  }
}

#[derive(Debug)]
#[derive(Clone)]
pub enum Direction {
  Horizontal,
  Vertical,
}
