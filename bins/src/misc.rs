use std::io::Read;
use std::time::SystemTime;
use std::{cmp::min, fmt, fs, io};

use lazy_static::lazy_static;
use libc::{
    c_char, c_int, c_long, c_void, fclose, feof, ferror, fgetc, fopen, fread, freopen, fseek,
    ftell, size_t, FILE,
};
use rand::prelude::*;

use ckb_vm::{
    registers::{A0, A1, A2, A3, A7},
    Error, Memory, Register, SupportMachine, Syscalls,
};

pub const READ_SYSCALL_NUMBER: u64 = 9000;
pub const NOW_SYSCALL_NUMBER: u64 = 9001;
pub const RANDOM_SYSCALL_NUMBER: u64 = 9002;
pub const FOPEN_SYSCALL_NUMBER: u64 = 9003;
pub const FREOPEN_SYSCALL_NUMBER: u64 = 9004;
pub const FREAD_SYSCALL_NUMBER: u64 = 9005;
pub const FEOF_SYSCALL_NUMBER: u64 = 9006;
pub const FERROR_SYSCALL_NUMBER: u64 = 9007;
pub const FGETC_SYSCALL_NUMBER: u64 = 9008;
pub const FCLOSE_SYSCALL_NUMBER: u64 = 9009;
pub const FTELL_SYSCALL_NUMBER: u64 = 9010;
pub const FSEEK_SYSCALL_NUMBER: u64 = 9011;

// TODO: fprintf, fwrite

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
        machine.set_register(A0, Mac::REG::from_u64(now as u64));
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
        machine.set_register(A0, Mac::REG::from_u64(r));
        return Ok(true);
    }
}

pub struct FileOperation {}

impl FileOperation {
    pub fn new() -> Self {
        Self {}
    }
    fn fetch_string<Mac: SupportMachine>(machine: &mut Mac, addr: u64) -> Result<String, Error> {
        let mut res = Vec::<u8>::new();
        let mut done = false;
        let mut count = 0;
        let mut addr = addr;
        while !done {
            let reg = Mac::REG::from_u64(addr);
            let eight_bytes = machine.memory_mut().load64(&reg)?;
            let buf = eight_bytes.to_u64().to_le_bytes();
            for c in buf {
                if c != 0 {
                    res.push(c);
                } else {
                    res.push(c);
                    done = true;
                    break;
                }
            }
            count += 1;
            if count > 1024 {
                panic!("too long string");
            }
            addr += 8;
        }
        Ok(String::from_utf8(res).expect("A valid UTF-8 string"))
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for FileOperation {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }
    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        let id = machine.registers()[A7].to_u64();
        let arg0 = machine.registers()[A0].to_u64();
        let arg1 = machine.registers()[A1].to_u64();
        let arg2 = machine.registers()[A2].to_u64();
        let arg3 = machine.registers()[A3].to_u64();

        match id {
            FOPEN_SYSCALL_NUMBER => {
                let path = Self::fetch_string(machine, arg0)?;
                let mode = Self::fetch_string(machine, arg1)?;
                let handler = unsafe {
                    fopen(
                        path.as_bytes().as_ptr() as *const c_char,
                        mode.as_bytes().as_ptr() as *const c_char,
                    )
                };
                machine.set_register(A0, Mac::REG::from_u64(handler as u64));
            }
            FREOPEN_SYSCALL_NUMBER => {
                let path = Self::fetch_string(machine, arg0)?;
                let mode = Self::fetch_string(machine, arg1)?;
                let stream = arg2;
                let handler = unsafe {
                    freopen(
                        path.as_bytes().as_ptr() as *const c_char,
                        mode.as_bytes().as_ptr() as *const c_char,
                        stream as *mut FILE,
                    )
                };
                machine.set_register(A0, Mac::REG::from_u64(handler as u64));
            }
            FREAD_SYSCALL_NUMBER => {
                let ptr = arg0;
                let size = arg1;
                let nitems = arg2;
                let stream = arg3;
                let total_size = nitems * size;
                if total_size > 3 * 1024 * 1024 {
                    panic!("too much memory to read");
                }
                let buf = vec![0u8; total_size as usize];
                let read_count = unsafe {
                    fread(
                        buf.as_ptr() as *mut c_void,
                        size as size_t,
                        nitems as size_t,
                        stream as *mut FILE,
                    )
                };
                machine
                    .memory_mut()
                    .store_bytes(ptr, &buf[0..read_count * size as usize])?;
                machine.set_register(A0, Mac::REG::from_u64(read_count as u64));
            }
            FEOF_SYSCALL_NUMBER => {
                let eof = unsafe { feof(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(eof));
            }
            FERROR_SYSCALL_NUMBER => {
                let error = unsafe { ferror(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(error));
            }
            FGETC_SYSCALL_NUMBER => {
                let ch = unsafe { fgetc(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(ch));
            }
            FCLOSE_SYSCALL_NUMBER => {
                let ret = unsafe { fclose(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(ret));
            }
            FTELL_SYSCALL_NUMBER => {
                let pos = unsafe { ftell(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i64(pos));
            }
            FSEEK_SYSCALL_NUMBER => {
                let ret = unsafe { fseek(arg0 as *mut FILE, arg1 as c_long, arg2 as c_int) };
                machine.set_register(A0, Mac::REG::from_i32(ret));
            }
            _ => {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
