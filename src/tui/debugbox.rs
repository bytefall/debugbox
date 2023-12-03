use anyhow::Result;
use std::rc::Rc;
use zi::prelude::*;

use crate::{
	bus::{Proxy, Regs},
	tui::{
		code::{Code, Properties as CodeProperties},
		data::{Data, Properties as DataProperties},
		registers::Registers,
		status_bar::{Status, StatusBar},
	},
};

pub enum Message {
	Reload,
	Run,
	StepOver,
	TraceInto,
}

pub struct DebugBox {
	frame: Rect,
	link: ComponentLink<Self>,
	proxy: Rc<Proxy>,
	status: Status,
	regs: Regs,
}

impl Component for DebugBox {
	type Message = Message;
	type Properties = Proxy;

	fn create(proxy: Self::Properties, frame: Rect, link: ComponentLink<Self>) -> Self {
		// TODO: need to handle timeout with `monitor_activity` (Call failed: Connection timed out)
		let (regs, status) = match proxy.regs.get() {
			Ok(r) => (r, Status::Attached),
			Err(e) => (Default::default(), Status::Detached(Some(e.to_string()))),
		};

		Self {
			frame,
			link,
			proxy: Rc::new(proxy),
			status,
			regs,
		}
	}

	fn update(&mut self, message: Self::Message) -> ShouldRender {
		let mut update = || -> Result<bool> {
			match message {
				Message::Reload => {
					self.regs = self.proxy.regs.get()?;
					self.status = Status::Attached;

					Ok(true)
				}
				Message::Run => {
					if self.status != Status::Attached {
						return Ok(false);
					}

					self.proxy.cpu.run()?;
					self.status = Status::Detached(None);

					Ok(true)
				}
				Message::StepOver => Ok(true),
				Message::TraceInto => Ok(true),
			}
		};

		update()
			.unwrap_or_else(|e| {
				self.status = Status::Detached(Some(e.to_string()));
				true
			})
			.into()
	}

	fn bindings(&self, bindings: &mut Bindings<Self>) {
		if !bindings.is_empty() {
			return;
		}

		bindings.set_focus(true);

		bindings.command("reload", || Message::Reload).with([Key::Ctrl('r')]);
		bindings.command("run", || Message::Run).with([Key::F(5)]);
		bindings.command("step-over", || Message::StepOver).with([Key::F(10)]);
		bindings.command("trace-into", || Message::TraceInto).with([Key::F(11)]);

		bindings
			.command("exit", |this: &Self| this.link.exit())
			.with([Key::Ctrl('c')]);
	}

	fn view(&self) -> Layout {
		const REGISTERS_WIDTH: usize = 50;

		Layout::column([
			Item::auto(Layout::row([
				Item::fixed(self.frame.size.width - REGISTERS_WIDTH - 1)(Layout::column([
					Item::auto(Code::with(CodeProperties {
						proxy: self.proxy.clone(),
						attached: self.status == Status::Attached,
						cs: self.regs.cs,
						eip: self.regs.eip,
					})),
					Item::auto(Data::with(DataProperties {
						proxy: self.proxy.clone(),
						attached: self.status == Status::Attached,
						addr: (self.regs.ds, 0).into(),
					})),
				])),
				Item::fixed(REGISTERS_WIDTH)(Registers::with(self.regs)),
			])),
			Item::fixed(1)(StatusBar::with(self.status.clone())),
		])
	}
}
