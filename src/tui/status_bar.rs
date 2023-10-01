use zi::{
	components::text::{Text, TextProperties},
	prelude::*,
};

const ATTACHED: char = '🟢';
const DETACHED: char = '⛔';
const STYLE: Style = Style::normal(super::BG_GRAY, super::FG_GRAY);

#[derive(Clone, PartialEq)]
pub enum Status {
	Attached,
	Detached(Option<String>),
}

pub struct StatusBar {
	status: Status,
}

impl Component for StatusBar {
	type Message = ();
	type Properties = Status;

	fn create(status: Self::Properties, _: Rect, _: ComponentLink<Self>) -> Self {
		Self { status }
	}

	fn change(&mut self, status: Self::Properties) -> ShouldRender {
		if self.status != status {
			self.status = status;

			true
		} else {
			false
		}
		.into()
	}

	fn view(&self) -> Layout {
		let tc = TextProperties::new().style(STYLE);
		let tc = match &self.status {
			Status::Attached => tc.content(format!(" {ATTACHED} Ready")),
			Status::Detached(None) => tc.content(format!(" {DETACHED} Detached")),
			Status::Detached(Some(reason)) => tc.content(format!(" {DETACHED} {reason}")),
		};

		Text::with_key("status-bar", tc)
	}
}
