use std::{
  fs::OpenOptions,
  time::{SystemTime, UNIX_EPOCH},
};

use tracing::Level;
use tracing_subscriber::{
  filter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

pub fn init_logger() -> anyhow::Result<()> {
  let current_ts = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs_f64();
  std::fs::create_dir_all("logs")?;
  let file = OpenOptions::new()
    .create(true)
    .truncate(true)
    .write(true)
    .append(false)
    .open(format!("logs/{}.log", current_ts))?;

  let targets_filter = filter::Targets::new().with_target("hat", Level::TRACE);
  let layer = tracing_subscriber::fmt::layer()
    .with_ansi(false)
    .with_line_number(true)
    .with_thread_names(true)
    .with_target(true)
    .with_writer(file);

  tracing_subscriber::registry()
    .with(layer)
    .with(targets_filter)
    .init();

  Ok(())
}
