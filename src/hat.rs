use std::env;

use dene::{
  AppContext, Context,
  panel::{Direction, Panel, PanelNode},
  view::{Interactive, Render},
  window::Window,
};
use qalam::{Qalam, multiaddr, utils::keypair::load_keypair_from};
use tokio::sync::mpsc;

use crate::{
  state::{CurrentPersona, CurrentTopic},
  ui::{Chat, Empty, InfoBlock, InputMessage},
  utils,
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
    let ident =
      load_keypair_from(&identities_path, &utils::path::sanitize(&persona.0))?;

    let (cmd_tx, cmd_rx) = mpsc::channel(64);
    let (event_tx, event_rx) = mpsc::channel(64);
    let listen_on = multiaddr!(Ip4([127, 0, 0, 1]), Tcp(32768u16));
    let qalam = Qalam::new(cmd_rx, event_tx, ident, listen_on, None);

    cx.spawn_on_background(async {
      qalam.start().await;
    })
    .detach();

    let global_topic = CurrentTopic::new("global".into());
    cx.set_global(global_topic.clone());
    // let global_topic = IdentTopic::new(&**global_topic);
    // swarm.behaviour_mut().gossipsub.subscribe(&global_topic)?;

    let info_block_id = window.next_pane_id();
    let info_block = cx.new_entity(|cx| InfoBlock::new(window, cx));

    let input_id = window.next_pane_id();
    let input = cx.new_entity(|cx| InputMessage::new(window, cx));

    let chat_id = window.next_pane_id();
    let chat = cx.new_entity(|cx| Chat::new(window, cx));

    // let _chat = chat.clone();
    // cx.spawn(async move |cx| {
    //   loop {
    //     tokio::select! {
    //       Some(event) = swarm.next() => {
    //         Self::handle_network_event(event, &mut swarm, &_chat, cx)?;
    //       }
    //       Some(cmd) = cmd_rx.recv() => {
    //         if let Err(err) = Self::handle_network_command(cmd, &mut swarm) {
    //           tracing::error!("cmd_rx error: {:?}", err);
    //         };
    //       }

    //       else => break,
    //     }
    //   }

    //   anyhow::Ok(())
    // })
    // .detach();

    // cx.subscribe(input.clone(), move |_, event, cx| {
    //   match event {
    //     InputEvent::Submit { data } => {
    //       let current_topic = cx.global::<CurrentTopic>();

    //       if let Err(err) = cmd_tx.send(HatNetworkCommand::SendMessage {
    //         topic: current_topic.0.clone(),
    //         message: data.clone(),
    //       }) {
    //         tracing::error!("[Hat] InputEvent subscriber: {:#?}", err);
    //       };
    //     }
    //   };
    // });

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
    window.active_pane.replace(input_id);

    Ok(Self {})
  }

  // fn handle_network_event(
  //   event: SwarmEvent<HatNetworkEvent>,
  //   swarm: &mut Swarm<HatNetwork>,
  //   chat: &Entity<Chat>,
  //   cx: &mut AsyncApp,
  // ) -> anyhow::Result<()> {
  //   let cx = cx.app();
  //   let mut cx = cx.borrow_mut();

  //   #[expect(clippy::single_match, reason = "")]
  //   match event {
  //     SwarmEvent::Behaviour(event) => match event {
  //       HatNetworkEvent::Mdns(event) => match event {
  //         mdns::Event::Discovered(peers) => {
  //           for (peer_id, addr) in peers.into_iter() {
  //             swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
  //             swarm.add_external_address(addr.clone());
  //             swarm.add_peer_address(peer_id, addr);
  //           }
  //         }
  //         mdns::Event::Expired(peers) => {
  //           for (peer_id, addr) in peers.into_iter() {
  //             swarm
  //               .behaviour_mut()
  //               .gossipsub
  //               .remove_explicit_peer(&peer_id);
  //             swarm.remove_external_address(&addr);
  //             swarm.disconnect_peer_id(peer_id).unwrap_or_else(|_| {
  //               tracing::warn!("failed to disconnect {:?}", peer_id);
  //             });
  //           }
  //         }
  //       },
  //       HatNetworkEvent::Gossipsub(event) => match event {
  //         gossipsub::Event::Message {
  //           propagation_source: _,
  //           message_id,
  //           message,
  //         } => {
  //           tracing::debug!("from id: {}; message: {:?}", message_id, message);

  //           let text = String::from_utf8(message.data).unwrap_or_default();
  //           let text = text.lines().map(Into::into).collect();

  //           cx.update_entity(chat, |_, cx| {
  //             cx.emit(ChatEvent::NewMessage {
  //               topic: message.topic,
  //               message: text,
  //             });
  //           });
  //         }
  //         _ => {}
  //       },
  //     },

  //     _ => {}
  //   };

  //   Ok(())
  // }

  // fn handle_network_command(
  //   cmd: HatNetworkCommand,
  //   swarm: &mut Swarm<HatNetwork>,
  // ) -> anyhow::Result<()> {
  //   match cmd {
  //     HatNetworkCommand::SendMessage { topic, message } => {
  //       let topic = IdentTopic::new(&*topic);
  //       swarm
  //         .behaviour_mut()
  //         .gossipsub
  //         .publish(topic.hash(), message)?;
  //     }
  //   };
  //   Ok(())
  // }
}
impl Render for Hat {}
impl Interactive for Hat {}
