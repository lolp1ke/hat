use std::sync::Arc;

use dene::{
  App, AppContext as _,
  executor::{BackgroundExecutor, ForegroundExecutor},
  keybind::KeybindsFile,
  ratatui::layout::Rect,
  window::WindowConfig,
};
use hat::{args::Args, state::CurrentPersona, ui::Hat};
use tokio::sync::mpsc;

fn main() -> anyhow::Result<()> {
  hat::utils::log::init_logger()?;
  let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?;

  let (width, height) = dene::ratatui::crossterm::terminal::size().unwrap();
  let terminal_area = Rect {
    x: 0,
    y: 0,
    width,
    height,
  };
  let Args { persona } = Args::new();

  let (tx, rx) = mpsc::unbounded_channel();
  let foreground_executor = ForegroundExecutor::new(tx);

  let multi_thread_handle = Arc::new(rt.handle().clone());
  let background_executor = BackgroundExecutor::new(multi_thread_handle);
  let app = App::new(foreground_executor, background_executor);

  rt.block_on(async {
    App::run(app.clone(), rx, move |cx| {
      let keybindings =
        KeybindsFile::parse(include_str!("../assets/default_keymap.toml"), cx)?;
      cx.load_keybinds(keybindings);
      cx.set_global(CurrentPersona::new(persona));

      let hat = cx.open_window(
        WindowConfig {
          area: terminal_area,
        },
        |window, cx| cx.new_entity(|cx| Hat::try_new(window, cx).unwrap()),
      );

      tracing::debug!("hat: {:?}", hat);

      anyhow::Ok(())
    })
    .await??;

    anyhow::Ok(())
  })?;

  tracing::debug!("{:#?}", app);

  Ok(())
}
