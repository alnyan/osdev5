use crate::sync::IrqSafeSpinLock;
use core::fmt;

#[derive(Debug)]
pub struct Config {
    cmdline: ConfigString<256>,
    console: ConfigString<16>,
    mem_limit: usize,
    initrd_base: usize,
    initrd_size: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum ConfigKey {
    Cmdline,
    Console,
    MemLimit,
    InitrdBase,
    InitrdSize
}

struct ConfigString<const N: usize> {
    buf: [u8; N],
    len: usize,
}

pub static CONFIG: IrqSafeSpinLock<Config> = IrqSafeSpinLock::new(Config::default());

impl const Default for Config {
    fn default() -> Self {
        Self {
            cmdline: ConfigString::empty(),
            console: ConfigString::empty(),
            mem_limit: usize::MAX,
            initrd_base: 0,
            initrd_size: 0
        }
    }
}

impl Config {
    pub fn set_usize(&mut self, key: ConfigKey, value: usize) {
        match key {
            ConfigKey::InitrdBase => { self.initrd_base = value }
            ConfigKey::InitrdSize => { self.initrd_size = value }
            ConfigKey::MemLimit => { self.mem_limit = value }
            _ => panic!("Invalid usize key: {:?}", key)
        }
    }

    pub fn set_str(&mut self, key: ConfigKey, value: &str) {
        match key {
            ConfigKey::Cmdline => { self.cmdline.set_from_str(value) }
            _ => panic!("Invalid str key: {:?}", key)
        }
    }

    pub fn get_usize(&self, key: ConfigKey) -> usize {
        match key {
            ConfigKey::InitrdBase => self.initrd_base,
            ConfigKey::InitrdSize => self.initrd_size,
            ConfigKey::MemLimit => self.mem_limit,
            _ => panic!("Invalid usize key: {:?}", key)
        }
    }

    pub fn get_str(&self, key: ConfigKey) -> &str {
        match key {
            ConfigKey::Cmdline => self.cmdline.as_str(),
            ConfigKey::Console => self.console.as_str(),
            _ => panic!("Invalid str key: {:?}", key)
        }
    }

    pub fn set_cmdline(&self, _cmdline: &str) {
        // TODO
    }
}

impl<const N: usize> ConfigString<N> {
    pub const fn empty() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap()
    }

    pub fn set_from_str(&mut self, data: &str) {
        let bytes = data.as_bytes();
        self.buf[..bytes.len()].copy_from_slice(bytes);
        self.len = bytes.len();
    }
}

impl<const N: usize> fmt::Debug for ConfigString<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}
