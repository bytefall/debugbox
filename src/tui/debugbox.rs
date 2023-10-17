use std::rc::Rc;

use anyhow::Result;
use zbus::blocking::Connection;
use zi::prelude::*;

use crate::tui::code::{Code, Properties as CodeProperties};
use crate::tui::registers::Registers;
use crate::tui::status_bar::{Status, StatusBar};
use crate::x86::cpu::Cpu;

pub enum Message {
	Reload,
	Run,
	StepOver,
	TraceInto,
}

pub struct DebugBox {
	frame: Rect,
	link: ComponentLink<Self>,
	conn: Rc<Connection>,
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

	fn create(conn: Self::Properties, frame: Rect, link: ComponentLink<Self>) -> Self {
		let mut this = Self {
			frame,
			link,
			conn: Rc::new(conn),
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
		const REGISTERS_WIDTH: usize = 50;

		Layout::column([
			Item::auto(Layout::row([
				Item::fixed(self.frame.size.width - REGISTERS_WIDTH - 1)(Code::with(CodeProperties {
					conn: self.conn.clone(),
					attached: self.status == Status::Attached,
					cs: self.cpu.regs.cs,
					eip: self.cpu.regs.eip,
				})),
				Item::fixed(REGISTERS_WIDTH)(Registers::with(self.cpu.regs)),
			])),
			Item::fixed(1)(StatusBar::with(self.status.clone())),
		])
	}
}
