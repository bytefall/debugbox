use anyhow::Result;
use std::rc::Rc;
use zi::{
	components::border::{Border, BorderProperties, BorderStroke},
	prelude::*,
};

use crate::{
	bus::{Proxy, Regs},
	tui::{
		code::{Code, Properties as CodeProperties},
		data::{Data, Properties as DataProperties},
		registers::Registers,
		status_bar::{Status, StatusBar},
	},
};

const FG_SELECTED: Colour = Colour::rgb(0, 255, 0);
const BORDER_NORMAL: Style = Style::normal(super::BG_DARK, super::FG_GRAY);
const BORDER_SELECTED: Style = Style::normal(super::BG_DARK, FG_SELECTED);
const BORDER_STROKE: BorderStroke = BorderStroke::heavy();

#[derive(Default, PartialEq)]
pub enum Pane {
	#[default]
	Code,
	Data,
	Registers,
}

pub enum Message {
	Reload,
	Run,
	StepOver,
	TraceInto,
	ChangePane(Pane),
}

pub struct DebugBox {
	frame: Rect,
	link: ComponentLink<Self>,
	pane: Pane,
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
			pane: Default::default(),
			proxy: Rc::new(proxy),
			status,
			regs,
		}
	}

	fn update(&mut self, message: Self::Message) -> ShouldRender {
		let update = || -> Result<bool> {
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
				Message::ChangePane(pane) => {
					self.pane = pane;

					Ok(true)
				}
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
			.command("code-pane", || Message::ChangePane(Pane::Code))
			.with([Key::Alt('1')]);
		bindings
			.command("data-pane", || Message::ChangePane(Pane::Data))
			.with([Key::Alt('2')]);
		bindings
			.command("registers-pane", || Message::ChangePane(Pane::Registers))
			.with([Key::Alt('3')]);

		bindings
			.command("exit", |this: &Self| this.link.exit())
			.with([Key::Ctrl('c')]);
	}

	fn view(&self) -> Layout {
		const REGISTERS_WIDTH: usize = 50;

		let code = CodeProperties {
			proxy: self.proxy.clone(),
			attached: self.status == Status::Attached,
			focused: self.pane == Pane::Code,
			cs: self.regs.cs,
			eip: self.regs.eip,
		};

		let data = DataProperties {
			proxy: self.proxy.clone(),
			attached: self.status == Status::Attached,
			focused: self.pane == Pane::Data,
			addr: (self.regs.ds, 0).into(),
		};

		let regs = self.regs;

		Layout::column([
			Item::auto(Layout::row([
				Item::fixed(self.frame.size.width - REGISTERS_WIDTH - 1)(Layout::column([
					Item::auto(create_pane(
						"code",
						"Alt-1",
						move || Code::with(code.clone()),
						self.pane == Pane::Code,
					)),
					Item::auto(create_pane(
						"data",
						"Alt-2",
						move || Data::with(data.clone()),
						self.pane == Pane::Data,
					)),
				])),
				Item::fixed(REGISTERS_WIDTH)(create_pane(
					"regs",
					"Alt-3",
					move || Registers::with(regs),
					self.pane == Pane::Registers,
				)),
			])),
			Item::fixed(1)(StatusBar::with(self.status.clone())),
		])
	}
}

fn create_pane(key: &str, title: &str, component: impl Fn() -> Layout + 'static, active: bool) -> Layout {
	let style = if active { BORDER_SELECTED } else { BORDER_NORMAL };

	let bp: BorderProperties = BorderProperties::new(component)
		.stroke(BORDER_STROKE)
		.title(Some((title, style)));

	Border::with_key(key, bp.style(style))
}
