use anyhow::Result;
use zbus::blocking::Connection;
use zi::prelude::*;

mod bus;
mod tui;
mod x86;

use crate::{bus::Proxy, tui::debugbox::DebugBox};

fn main() -> Result<()> {
    let app = DebugBox::with(Proxy::new(&Connection::session()?)?);

    zi_term::incremental()?.run_event_loop(app)?;

    Ok(())
}
