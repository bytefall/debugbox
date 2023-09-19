use serde::Deserialize;
use zbus::zvariant::Type;

#[derive(Copy, Clone, Default)]
pub struct Cpu {
	pub regs: Registers,
}

#[derive(Copy, Clone, Default, Deserialize, PartialEq, Type)]
pub struct Registers {
	pub eax: u32,
	pub ebx: u32,
	pub ecx: u32,
	pub edx: u32,
	pub esi: u32,
	pub edi: u32,
	pub ebp: u32,
	pub esp: u32,
	pub eip: u32,
	pub cs: u16,
	pub ds: u16,
	pub es: u16,
	pub _fs: u16,
	pub _gs: u16,
	pub ss: u16,
	pub _cf: bool,
	pub _pf: bool,
	pub _af: bool,
	pub zf: bool,
	pub _sf: bool,
	pub _tf: bool,
	pub _if: bool,
	pub _df: bool,
	pub _of: bool,
	pub _iopl: u8,
	pub _nt: bool,
	pub _vm: bool,
	pub _ac: bool,
	pub _id: bool,
}
