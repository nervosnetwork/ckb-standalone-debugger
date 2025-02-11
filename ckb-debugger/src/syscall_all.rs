use ckb_vm::{
    registers::{A0, A1, A2, A3, A7},
    Error, Memory, Register, SupportMachine, Syscalls,
};
use libc::{
    c_char, c_int, c_long, c_void, fclose, feof, ferror, fgetc, fopen, fread, freopen, fseek, ftell, fwrite, size_t,
    FILE,
};
use rand::prelude::*;
use std::ffi::CString;
use std::io::Read;
use std::time::SystemTime;
use std::{cmp::min, fs, io};

pub const SYSCALL_NUMBER_READ: u64 = 9000;
pub const SYSCALL_NUMBER_NOW: u64 = 9001;
pub const SYSCALL_NUMBER_RANDOM: u64 = 9002;
pub const SYSCALL_NUMBER_FOPEN: u64 = 9003;
pub const SYSCALL_NUMBER_FREOPEN: u64 = 9004;
pub const SYSCALL_NUMBER_FREAD: u64 = 9005;
pub const SYSCALL_NUMBER_FEOF: u64 = 9006;
pub const SYSCALL_NUMBER_FERROR: u64 = 9007;
pub const SYSCALL_NUMBER_FGETC: u64 = 9008;
pub const SYSCALL_NUMBER_FCLOSE: u64 = 9009;
pub const SYSCALL_NUMBER_FTELL: u64 = 9010;
pub const SYSCALL_NUMBER_FSEEK: u64 = 9011;
pub const SYSCALL_NUMBER_FWRITE: u64 = 9012;

pub struct FileOperation {}

impl FileOperation {
    pub fn new() -> Self {
        Self {}
    }
    fn fetch_string<Mac: SupportMachine>(machine: &mut Mac, addr: u64) -> Result<CString, Error> {
        let mut buffer = Vec::new();
        let mut addr = addr;
        loop {
            let byte = machine.memory_mut().load8(&Mac::REG::from_u64(addr))?;
            if byte.to_u8() == 0 {
                break;
            }
            buffer.push(byte.to_u8());
            addr += 1;
        }
        Ok(CString::new(buffer).expect("Invalid C string"))
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
            SYSCALL_NUMBER_FOPEN => {
                let path = Self::fetch_string(machine, arg0)?;
                let mode = Self::fetch_string(machine, arg1)?;
                let handler = unsafe {
                    fopen(
                        path.as_bytes_with_nul().as_ptr() as *const c_char,
                        mode.as_bytes_with_nul().as_ptr() as *const c_char,
                    )
                };
                machine.set_register(A0, Mac::REG::from_u64(handler as u64));
            }
            SYSCALL_NUMBER_FREOPEN => {
                let path = Self::fetch_string(machine, arg0)?;
                let mode = Self::fetch_string(machine, arg1)?;
                let stream = arg2;
                let handler = unsafe {
                    freopen(
                        path.as_bytes_with_nul().as_ptr() as *const c_char,
                        mode.as_bytes_with_nul().as_ptr() as *const c_char,
                        stream as *mut FILE,
                    )
                };
                machine.set_register(A0, Mac::REG::from_u64(handler as u64));
            }
            SYSCALL_NUMBER_FREAD => {
                let ptr = arg0;
                let size = arg1;
                let nitems = arg2;
                let stream = arg3;
                let total_size = nitems * size;
                if total_size > 3 * 1024 * 1024 {
                    panic!("Too much memory to read");
                }
                let buf = vec![0u8; total_size as usize];
                let read_count = unsafe {
                    fread(buf.as_ptr() as *mut c_void, size as size_t, nitems as size_t, stream as *mut FILE)
                };
                machine.memory_mut().store_bytes(ptr, &buf[0..read_count * size as usize])?;
                machine.set_register(A0, Mac::REG::from_u64(read_count as u64));
            }
            SYSCALL_NUMBER_FEOF => {
                let eof = unsafe { feof(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(eof));
            }
            SYSCALL_NUMBER_FERROR => {
                let error = unsafe { ferror(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(error));
            }
            SYSCALL_NUMBER_FGETC => {
                let ch = unsafe { fgetc(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(ch));
            }
            SYSCALL_NUMBER_FCLOSE => {
                let ret = unsafe { fclose(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i32(ret));
            }
            SYSCALL_NUMBER_FTELL => {
                let pos = unsafe { ftell(arg0 as *mut FILE) };
                machine.set_register(A0, Mac::REG::from_i64(pos.into()));
            }
            SYSCALL_NUMBER_FSEEK => {
                let ret = unsafe { fseek(arg0 as *mut FILE, arg1 as c_long, arg2 as c_int) };
                machine.set_register(A0, Mac::REG::from_i32(ret));
            }
            SYSCALL_NUMBER_FWRITE => {
                let ptr = arg0;
                let size = arg1;
                let nitems = arg2;
                let stream = arg3;
                let total_size = nitems * size;
                if total_size > 3 * 1024 * 1024 {
                    panic!("Too much memory to write");
                }
                let buf = machine.memory_mut().load_bytes(ptr, total_size)?;
                let write_count = unsafe {
                    fwrite(buf.as_ptr() as *mut c_void, size as size_t, nitems as size_t, stream as *mut FILE)
                };
                machine.set_register(A0, Mac::REG::from_u64(write_count as u64));
            }
            _ => {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[derive(Clone)]
pub struct FileStream {
    content: Vec<u8>,
    offset: usize,
}

impl Default for FileStream {
    fn default() -> Self {
        Self { content: Default::default(), offset: 0 }
    }
}

impl FileStream {
    pub fn new(file_name: &str) -> Self {
        let content = if file_name == "-" {
            let mut v = Vec::<u8>::new();
            let mut stdin = io::stdin();
            stdin.read_to_end(&mut v).expect("Should read from stdin");
            v
        } else {
            fs::read(file_name).expect("Should read the file")
        };
        FileStream { content, offset: 0 }
    }
    // Mimic: ssize_t read(int fd, void *buf, size_t count);
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
        if id != SYSCALL_NUMBER_READ {
            return Ok(false);
        }
        let arg_buf = machine.registers()[A0].to_u64();
        let arg_count = machine.registers()[A1].to_u64();
        let mut buf = vec![0u8; arg_count as usize];
        let read_size = self.read(&mut buf);
        if read_size > 0 {
            machine.memory_mut().store_bytes(arg_buf, &buf[0..read_size as usize])?;
            machine.set_register(A0, Mac::REG::from_u64(read_size as u64));
        } else {
            machine.set_register(A0, Mac::REG::from_i64(-1));
        }
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
        if id != SYSCALL_NUMBER_RANDOM {
            return Ok(false);
        }
        let r: u64 = random();
        machine.set_register(A0, Mac::REG::from_u64(r));
        return Ok(true);
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
        if id != SYSCALL_NUMBER_NOW {
            return Ok(false);
        }
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let now = duration.as_nanos();
        machine.set_register(A0, Mac::REG::from_u64(now as u64));
        return Ok(true);
    }
}
