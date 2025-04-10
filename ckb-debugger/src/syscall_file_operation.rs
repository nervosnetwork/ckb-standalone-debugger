#[cfg(any(target_family = "unix", target_family = "windows"))]
mod arch {
    use ckb_vm::{Error, SupportMachine, Syscalls};
    use ckb_vm::{
        Memory, Register,
        registers::{A0, A1, A2, A3, A7},
    };
    use libc::{
        FILE, c_char, c_int, c_long, c_void, fclose, feof, ferror, fgetc, fopen, fread, freopen, fseek, ftell, fwrite,
        size_t,
    };
    use std::ffi::CString;

    const SYSCALL_NUMBER_FOPEN: u64 = 9003;
    const SYSCALL_NUMBER_FREOPEN: u64 = 9004;
    const SYSCALL_NUMBER_FREAD: u64 = 9005;
    const SYSCALL_NUMBER_FEOF: u64 = 9006;
    const SYSCALL_NUMBER_FERROR: u64 = 9007;
    const SYSCALL_NUMBER_FGETC: u64 = 9008;
    const SYSCALL_NUMBER_FCLOSE: u64 = 9009;
    const SYSCALL_NUMBER_FTELL: u64 = 9010;
    const SYSCALL_NUMBER_FSEEK: u64 = 9011;
    const SYSCALL_NUMBER_FWRITE: u64 = 9012;

    pub struct FileOperation {}

    impl FileOperation {
        pub fn new() -> Self {
            Self {}
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
                    let path = CString::new(ckb_vm::memory::load_c_string_byte_by_byte(
                        machine.memory_mut(),
                        &Mac::REG::from_u64(arg0),
                    )?)
                    .expect("Invalid C string");
                    let mode = CString::new(ckb_vm::memory::load_c_string_byte_by_byte(
                        machine.memory_mut(),
                        &Mac::REG::from_u64(arg1),
                    )?)
                    .expect("Invalid C string");
                    let handler = unsafe {
                        fopen(
                            path.as_bytes_with_nul().as_ptr() as *const c_char,
                            mode.as_bytes_with_nul().as_ptr() as *const c_char,
                        )
                    };
                    machine.set_register(A0, Mac::REG::from_u64(handler as u64));
                }
                SYSCALL_NUMBER_FREOPEN => {
                    let path = CString::new(ckb_vm::memory::load_c_string_byte_by_byte(
                        machine.memory_mut(),
                        &Mac::REG::from_u64(arg0),
                    )?)
                    .expect("Invalid C string");
                    let mode = CString::new(ckb_vm::memory::load_c_string_byte_by_byte(
                        machine.memory_mut(),
                        &Mac::REG::from_u64(arg1),
                    )?)
                    .expect("Invalid C string");
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
}

#[cfg(target_family = "wasm")]
mod arch {
    use ckb_vm::{Error, SupportMachine, Syscalls};

    #[derive(Default)]
    pub struct FileOperation {}

    impl FileOperation {
        pub fn new() -> Self {
            FileOperation {}
        }
    }

    impl<Mac: SupportMachine> Syscalls<Mac> for FileOperation {
        fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
            Ok(())
        }

        fn ecall(&mut self, _machine: &mut Mac) -> Result<bool, Error> {
            Ok(false)
        }
    }
}

pub use arch::FileOperation;
