pub mod cpu;
pub mod dec;

#[derive(Copy, Clone)]
pub struct Address {
	pub segment: u16,
	pub offset: u32,
}

impl From<(u16, u32)> for Address {
	fn from((segment, offset): (u16, u32)) -> Self {
		Self { segment, offset }
	}
}
