use anyhow::Result;
use zbus::blocking::Connection;
use zi::prelude::*;

mod tui;
mod x86;

use crate::tui::debugbox::DebugBox;

fn main() -> Result<()> {
	let app = DebugBox::with(Connection::session()?);

	zi_term::incremental()?.run_event_loop(app)?;

	Ok(())
}
