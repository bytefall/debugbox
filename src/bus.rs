use serde::Deserialize;
use zbus::{blocking::Connection, dbus_proxy, zvariant::Type, Result};

pub struct Proxy {
    pub cpu: CpuProxyBlocking<'static>,
    pub regs: RegsProxyBlocking<'static>,
    pub mem: MemoryProxyBlocking<'static>,
}

impl Proxy {
    pub fn new(conn: &Connection) -> Result<Self> {
        Ok(Self {
            cpu: CpuProxyBlocking::new(conn)?,
            regs: RegsProxyBlocking::new(conn)?,
            mem: MemoryProxyBlocking::new(conn)?,
        })
    }
}

#[dbus_proxy(
    interface = "com.dosbox",
    default_service = "com.dosbox",
    default_path = "/cpu"
)]
trait Cpu {
    #[dbus_proxy(name = "get")]
    fn get(&self) -> Result<(bool, bool)>;

    #[dbus_proxy(name = "callback_info")]
    fn callback_info(&self, index: u16) -> Result<String>;

    #[dbus_proxy(name = "step_in")]
    fn step_in(&self) -> Result<u32>;

    #[dbus_proxy(name = "run")]
    fn run(&self) -> Result<u32>;
}

#[dbus_proxy(
    interface = "com.dosbox",
    default_service = "com.dosbox",
    default_path = "/cpu/regs"
)]
trait Regs {
    #[dbus_proxy(name = "get")]
    fn get(&self) -> Result<Regs>;
}

#[dbus_proxy(
    interface = "com.dosbox",
    default_service = "com.dosbox",
    default_path = "/mem"
)]
trait Memory {
    #[dbus_proxy(name = "get")]
    fn get(&self, segment: u16, offset: u32, length: u32) -> Result<Vec<u8>>;

    #[dbus_proxy(name = "set")]
    fn set(&self, segment: u16, offset: u32, value: u8) -> Result<u8>;
}

#[derive(Copy, Clone, Default, Deserialize, PartialEq, Type)]
pub struct Regs {
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
    pub fs: u16,
    pub gs: u16,
    pub ss: u16,
    pub cf: bool,
    pub pf: bool,
    pub af: bool,
    pub zf: bool,
    pub sf: bool,
    pub tf: bool,
    pub r#if: bool,
    pub df: bool,
    pub of: bool,
    pub _iopl: u8,
    pub _nt: bool,
    pub _vm: bool,
    pub _ac: bool,
    pub _id: bool,
}
