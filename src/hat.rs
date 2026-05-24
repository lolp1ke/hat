// SPDX-License-Identifier: Apache-2.0

use std::env;

use dene::{
  AppContext, Context,
  panel::{Direction, Panel, PanelNode},
  view::{Interactive, Render},
  window::Window,
};
use qalam::{
  Qalam, multiaddr, room::RoomId, utils::keypair::load_keypair_from,
};
use tokio::sync::mpsc;

use crate::{
  state::{AddressBook, CurrentPersona, CurrentTopic},
  ui::{Chat, Empty, InfoBlock, InputEvent, InputMessage},
};

#[derive(Debug)]
pub struct Hat {}
impl Hat {
  pub fn try_new(
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> anyhow::Result<Self> {
    let persona = cx.global::<CurrentPersona>().clone();
    let cfg_path = env::home_dir().expect("no home?").join(".config/hat");
    let identities_path = cfg_path.join("identities");
    if !cfg_path.exists() || !identities_path.exists() {
      std::fs::create_dir_all(&identities_path)?;
    };
    let ident = load_keypair_from(&identities_path, &persona.0)?;

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (event_tx, mut event_rx) = mpsc::channel(64);
    let listen_on = multiaddr!(Ip4([0, 0, 0, 0]), Tcp(0u16));
    let qalam =
      Qalam::new(persona.0.clone(), cmd_rx, event_tx, ident, listen_on, None);
    let local_peer_id = qalam.local_peer_id();
    tracing::debug!("Current local_peer_id: {:?}", local_peer_id.to_string());

    let mut address_book =
      AddressBook::try_load(&cfg_path.join("address_book.toml"))
        .unwrap_or_default();
    address_book.insert(local_peer_id.to_string(), persona.0, true);
    cx.set_global(address_book);

    cx.spawn_on_background(async move {
      qalam.start().await;
    })
    .detach();

    let global_topic = CurrentTopic::new("global".into());
    cx.set_global(global_topic.clone());

    if let Err(err) = cmd_tx.send(qalam::command::QalamCommand::JoinRoom {
      name: global_topic.0.clone(),
    }) {
      tracing::warn!("failed to send command: {:?}\n{:?}", err.0, err);
    };

    let info_block_id = window.next_pane_id();
    let info_block = cx.new_entity(|cx| InfoBlock::new(window, cx));

    let input_id = window.next_pane_id();
    let input = cx.new_entity(|cx| InputMessage::new(window, cx));
    cx.subscribe(input.clone(), {
      let cmd_tx = cmd_tx.clone();
      move |_, event, cx| {
        use qalam::command::QalamCommand;
        let topic = cx.global::<CurrentTopic>().0.clone();

        match event {
          InputEvent::Submit { data } => {
            let room = RoomId::room(&topic);
            if let Err(err) = cmd_tx.send(QalamCommand::SendRoomMessage {
              room,
              from: local_peer_id,
              message: data.clone(),
            }) {
              tracing::warn!("failed to send command: {:?}\n{:?}", err.0, err);
            };
          }
        };
      }
    });

    let chat_id = window.next_pane_id();
    let chat = cx.new_entity(|cx| Chat::new(local_peer_id, window, cx));

    cx.spawn({
      let chat = chat.clone();
      async move |cx| {
        use qalam::event::QalamEvent;

        while let Some(event) = event_rx.recv().await {
          match event {
            QalamEvent::RoomLeft { room } => {}
            QalamEvent::ChatMessageReceieved { room, message } => {
              let from = message.from;
              cx.update_entity(&chat, |chat, cx| {
                chat.messages.entry(room).or_default().push(message);
              });

              let peer_str = from.to_string().clone();
              let known =
                cx.read_global::<AddressBook, _, _>(|address_book, _| {
                  address_book.persona_by_peer_id.contains_key(&*peer_str)
                });

              if !known {
                tracing::warn!(
                  "peer {} is not registed in address book.",
                  peer_str
                );

                if let Err(err) =
                  cmd_tx.send(qalam::command::QalamCommand::RequestPersona {
                    peer: from,
                  })
                {
                  tracing::warn!("failed to request persona: {:?}", err);
                };
              };
            }

            QalamEvent::PersonaReceived { peer, persona } => {
              let app = cx.app();
              let mut app = app.borrow_mut();
              let address_book = app.global_mut::<AddressBook>();
              address_book.insert(peer.to_string(), persona, true);
              // NOTE: just a reminder comment so i won't borrow [`cx`] while still having [`RefMut<'_, App>`]
              //       in case i add more code below: "use [`app`] or drop it and use [`cx`]"
              // drop(app);
            }

            _ => {}
          };
        }
      }
    })
    .detach();

    let empty_ids = [
      window.next_pane_id(),
      window.next_pane_id(),
      window.next_pane_id(),
    ];

    window.root.replace(PanelNode::Split {
      direction: Direction::Horizontal,
      children: vec![
        PanelNode::Split {
          direction: Direction::Vertical,
          children: vec![
            PanelNode::Leaf(Panel {
              id: info_block_id,
              view: info_block.into(),
            }),
            PanelNode::Split {
              direction: Direction::Vertical,
              children: vec![
                PanelNode::Leaf(Panel {
                  id: chat_id,
                  view: chat.into(),
                }),
                PanelNode::Leaf(Panel {
                  id: input_id,
                  view: input.into(),
                }),
              ],
              weights: vec![6.0, 1.0],
            },
          ],
          weights: vec![1.0, 6.0],
        },
        PanelNode::Leaf(Panel {
          id: empty_ids[2],
          view: cx.new_entity(|_| Empty).into(),
        }),
      ],
      weights: vec![5.0, 1.0],
    });
    window.active_panel.replace(input_id);

    Ok(Self {})
  }
}
impl Render for Hat {}
impl Interactive for Hat {}
