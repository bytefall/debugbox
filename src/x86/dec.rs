use anyhow::{anyhow, Result};
use capstone::prelude::*;
use std::rc::Rc;
use zbus::blocking::Connection;

use super::Address;

const AVG_INSTR_LEN: usize = 3; // average instruction length
const MAX_INSTR_LEN: usize = 15; // maximal instruction length (don't load less than that)
const PRE_INSTR_LEN: usize = MAX_INSTR_LEN * 4; // number of bytes to pre-load when scrolling top. Should not be too small to give a change to decode few instruction(s).

pub struct Decoder {
	conn: Rc<Connection>,
	caps: Capstone,
}

impl Decoder {
	pub fn new(conn: Rc<Connection>) -> Self {
		let caps = Capstone::new()
			.x86()
			.mode(arch::x86::ArchMode::Mode16)
			.syntax(arch::x86::ArchSyntax::Intel)
			.detail(true)
			.build()
			.unwrap();

		Self { conn, caps }
	}

	pub fn before(&self, addr: Address, limit: usize) -> Result<Vec<Instruction>> {
		let mut code: Vec<Instruction> = Vec::new();

		while code.len() < limit {
			let first = code.first().map(|i| i.offset).unwrap_or(addr.offset);

			let start =
				first.saturating_sub((PRE_INSTR_LEN + (limit.saturating_sub(code.len()) * AVG_INSTR_LEN)) as u32);

			let data: Vec<u8> = self
				.conn
				.call_method(
					Some("com.dosbox"),
					"/mem",
					Some("com.dosbox"),
					"get",
					&(addr.segment, start, first.saturating_sub(start)),
				)?
				.body_unchecked()?;

			// start of the segment
			if start == 0 {
				let mut data = self.disasm(&data, start)?;
				data.append(&mut code);

				return Ok(data);
			}

			let Some(mut data) = (0..PRE_INSTR_LEN).rev().find_map(|i| {
				let (h, t) = data.split_at(i);

				let (Ok(h), Ok(t)) = (self.disasm(h, start), self.disasm(t, start + h.len() as u32)) else {
					return None;
				};

				// unable to decode tail
				if t.is_empty() {
					return None;
				}

				// bytes missing in between (i.e. head[offset + len] != tail[offset])
				if h.last().map(|x| x.offset + x.data.len() as u32) != t.first().map(|x| x.offset) {
					return None;
				}

				Some(t)
			}) else {
				break;
			};

			data.append(&mut code);
			code = data;
		}

		Ok(code)
	}

	pub fn after(&self, addr: Address, limit: usize) -> Result<Vec<Instruction>> {
		let mut code: Vec<Instruction> = Vec::new();

		while code.len() < limit {
			let start = code
				.last()
				.map(|i| i.offset + i.data.len() as u32)
				.unwrap_or(addr.offset);

			let end = start.saturating_add(MAX_INSTR_LEN.max(limit.saturating_sub(code.len()) * AVG_INSTR_LEN) as u32);

			let data: Vec<u8> = self
				.conn
				.call_method(
					Some("com.dosbox"),
					"/mem",
					Some("com.dosbox"),
					"get",
					&(addr.segment, start, end.saturating_sub(start)),
				)?
				.body_unchecked()?;

			let data = self.disasm(&data, start)?;

			if data.is_empty() {
				break;
			}

			code.extend(data);
		}

		Ok(code)
	}

	fn disasm(&self, data: &[u8], start: u32) -> Result<Vec<Instruction>> {
		Ok(self
			.caps
			.disasm_all(data, start.into())
			.map_err(|e| anyhow!("{e}"))?
			.iter()
			.map(|i| Instruction {
				offset: i.address() as u32,
				data: i.bytes().to_vec(),
				mnemonic: i.mnemonic().map(String::from),
				operands: i.op_str().map(String::from),
			})
			.collect())
	}
}

pub struct Instruction {
	pub offset: u32,
	pub data: Vec<u8>,
	pub mnemonic: Option<String>,
	pub operands: Option<String>,
}
