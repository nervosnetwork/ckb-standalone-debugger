use lazy_static::lazy_static;
use rand::prelude::*;
use std::io::Read;
use std::time::{SystemTime};
use std::{cmp::min, fmt, fs, io};

use ckb_vm::{
    registers::{A0, A1, A7},
    Error, Memory, Register, SupportMachine, Syscalls,
};

pub const READ_SYSCALL_NUMBER: u64 = 9000;
pub const NOW_SYSCALL_NUMBER: u64 = 9001;
pub const RANDOM_SYSCALL_NUMBER: u64 = 9002;

#[derive(Clone)]
pub struct FileStream {
    content: Vec<u8>,
    offset: usize,
}

lazy_static! {
    static ref STREAM: FileStream = Default::default();
}

impl Default for FileStream {
    fn default() -> Self {
        Self {
            content: Default::default(),
            offset: 0,
        }
    }
}

impl FileStream {
    pub fn new(file_name: &str) -> Self {
        let s = if file_name == "-" {
            let mut s = String::new();
            let mut stdin = io::stdin();
            stdin
                .read_to_string(&mut s)
                .expect("should read from stdin");
            s
        } else {
            fs::read_to_string(file_name).expect("should read the file")
        };
        FileStream {
            content: s.into_bytes(),
            offset: 0,
        }
    }
    // mimic:  ssize_t read(int fd, void *buf, size_t count);
    fn read(&mut self, buf: &mut [u8]) -> isize {
        if self.offset >= self.content.len() {
            return -1;
        }
        let remaining_size = self.content.len() - self.offset;
        let read_size = min(buf.len(), remaining_size);
        buf[0..read_size].copy_from_slice(&self.content[self.offset..self.offset + read_size]);

        self.offset += read_size;
        read_size as isize
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for FileStream {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        let id = machine.registers()[A7].to_u64();
        if id != READ_SYSCALL_NUMBER {
            return Ok(false);
        }
        let arg_buf = machine.registers()[A0].to_u64();
        let arg_count = machine.registers()[A1].to_u64();
        let mut buf = vec![0u8; arg_count as usize];
        let read_size = self.read(&mut buf);
        if read_size > 0 {
            machine
                .memory_mut()
                .store_bytes(arg_buf, &buf[0..read_size as usize])?;
            machine.set_register(A0, Mac::REG::from_u64(read_size as u64));
        } else {
            machine.set_register(A0, Mac::REG::from_i64(-1));
        }
        return Ok(true);
    }
}

pub struct HumanReadableCycles(pub u64);

impl fmt::Display for HumanReadableCycles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;
        if self.0 >= 1024 * 1024 {
            write!(f, "({:.1}M)", self.0 as f64 / 1024. / 1024.)?;
        } else if self.0 >= 1024 {
            write!(f, "({:.1}K)", self.0 as f64 / 1024.)?;
        } else {
        }
        Ok(())
    }
}

pub struct TimeNow {}

impl TimeNow {
    pub fn new() -> Self {
        Self {}
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for TimeNow {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        let id = machine.registers()[A7].to_u64();
        if id != NOW_SYSCALL_NUMBER {
            return Ok(false);
        }
        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let now = duration.as_nanos();
        let buf = now.to_le_bytes();
        let arg_buf = machine.registers()[A0].to_u64();
        machine.memory_mut().store_bytes(arg_buf, &buf[..])?;
        return Ok(true);
    }
}

pub struct Random {}

impl Random {
    pub fn new() -> Self {
        Self {}
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for Random {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        let id = machine.registers()[A7].to_u64();
        if id != RANDOM_SYSCALL_NUMBER {
            return Ok(false);
        }
        let r: u64 = random();
        let buf = r.to_le_bytes();
        let arg_buf = machine.registers()[A0].to_u64();
        machine.memory_mut().store_bytes(arg_buf, &buf[..])?;
        return Ok(true);
    }
}
