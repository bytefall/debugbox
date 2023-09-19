use anyhow::Result;
use zbus::blocking::Connection;
use zi::prelude::*;

use crate::cpu::Cpu;
use crate::tui::code::{Code, Properties as CodeProperties};
use crate::tui::registers::Registers;
use crate::tui::status_bar::{Status, StatusBar};

pub enum Message {
	Reload,
	Run,
	StepOver,
	TraceInto,
}

pub struct DebugBox {
	link: ComponentLink<Self>,
	conn: Connection,
	status: Status,
	cpu: Cpu,
}

impl DebugBox {
	fn update(&mut self) -> Result<()> {
		self.cpu.regs = self
			.conn
			.call_method(Some("com.dosbox"), "/cpu/regs", Some("com.dosbox"), "get", &())?
			.body_unchecked()?;

		Ok(())
	}

	fn run(&self) -> Result<()> {
		self.conn
			.call_method(Some("com.dosbox"), "/cpu", Some("com.dosbox"), "run", &())?;

		Ok(())
	}
}

impl Component for DebugBox {
	type Message = Message;
	type Properties = Connection;

	fn create(conn: Self::Properties, _frame: Rect, link: ComponentLink<Self>) -> Self {
		let mut this = Self {
			link,
			conn,
			status: Status::Detached(None),
			cpu: Default::default(),
		};

		// TODO: need to handle timeout with `monitor_activity` (Call failed: Connection timed out)
		this.status = if let Err(e) = this.update() {
			Status::Detached(Some(e.to_string()))
		} else {
			Status::Attached
		};

		this
	}

	fn update(&mut self, message: Self::Message) -> ShouldRender {
		let mut update = || -> Result<bool> {
			match message {
				Message::Reload => {
					self.update()?;
					self.status = Status::Attached;

					Ok(true)
				}
				Message::Run => {
					if self.status != Status::Attached {
						return Ok(false);
					}

					self.run()?;
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
			.with([Key::Ctrl('c')])
			.with([Key::Esc]);
	}

	fn view(&self) -> Layout {
		Layout::column([
			Item::auto(Layout::row([
				Item::auto(Code::with(CodeProperties {
					attached: self.status == Status::Attached,
					cs: self.cpu.regs.cs,
					eip: self.cpu.regs.eip,
				})),
				Item::fixed(50)(Registers::with(self.cpu.regs)),
			])),
			Item::fixed(1)(StatusBar::with(self.status.clone())),
		])
	}
}
