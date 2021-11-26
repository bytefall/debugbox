use iced::{button, Align, Button, Column, Element, Sandbox, Text};
use zbus::Connection;

#[derive(Default)]
pub struct DebugBox {
	cpu_run: button::State,
	console: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
	CpuRun,
}

impl Sandbox for DebugBox {
	type Message = Message;

	fn new() -> Self {
		Self::default()
	}

	fn title(&self) -> String {
		String::from("DebugBox")
	}

	fn update(&mut self, message: Self::Message) {
		match message {
			Message::CpuRun => {
				let conn = pollster::block_on(Connection::session()).unwrap();
				let result = pollster::block_on(conn.call_method(Some("com.dosbox"), "/cpu", Some("com.dosbox"), "run", &())).unwrap();

				self.console = format!("{:?}", result);
			}
		}
	}

	fn view(&mut self) -> Element<Self::Message> {
		Column::new()
			.padding(20)
			.align_items(Align::Center)
			.push(Button::new(&mut self.cpu_run, Text::new("Run")).on_press(Message::CpuRun))
			.push(Text::new(&self.console).size(12))
			.into()
	}
}
