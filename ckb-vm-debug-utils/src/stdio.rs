use ckb_vm::{
    registers::{A0, A1, A2, A7},
    Error, Memory, Register, SupportMachine, Syscalls,
};
use libc::{SEEK_CUR, SEEK_END, SEEK_SET};
use nix::{
    sys::stat::fstat,
    unistd::{close, lseek, read, write, Whence},
};
use std::mem::size_of;
use std::slice::from_raw_parts;

#[derive(Clone, Debug, Default)]
#[repr(C)]
struct AbiStat {
    dev: u64,
    ino: u64,
    mode: u32,
    nlink: i32,
    uid: u32,
    gid: u32,
    rdev: u64,
    __pad1: u64,
    size: i64,
    blksize: i32,
    __pad2: i32,
    blocks: i64,
    atime: i64,
    atime_nsec: i64,
    mtime: i64,
    mtime_nsec: i64,
    ctime: i64,
    ctime_nsec: i64,
    __unused4: i32,
    __unused5: i32,
}

#[derive(Default)]
pub struct Stdio {
    keep_stdios: bool,
}

impl Stdio {
    pub fn new(keep_stdios: bool) -> Self {
        Stdio { keep_stdios }
    }

    fn close<Mac: SupportMachine>(&mut self, machine: &mut Mac) -> Result<(), Error> {
        let fd = machine.registers()[A0].to_i32();
        if (fd >= 0 && fd <= 2) && self.keep_stdios {
            machine.set_register(A0, Mac::REG::zero());
            return Ok(());
        }
        let ret = match close(fd) {
            Ok(()) => 0,
            Err(e) => {
                debug!("close error: {:?}", e);
                (-1i64) as u64
            }
        };
        machine.set_register(A0, Mac::REG::from_u64(ret));
        Ok(())
    }

    fn fstat<Mac: SupportMachine>(&mut self, machine: &mut Mac) -> Result<(), Error> {
        let stat = match fstat(machine.registers()[A0].to_i32()) {
            Ok(stat) => stat,
            Err(e) => {
                debug!("fstat error: {:?}", e);
                machine.set_register(A0, Mac::REG::from_i8(-1));
                return Ok(());
            }
        };
        let mut abi_stat = AbiStat::default();
        abi_stat.dev = stat.st_dev as u64;
        abi_stat.ino = stat.st_ino;
        abi_stat.mode = stat.st_mode as u32;
        abi_stat.nlink = stat.st_nlink as i32;
        abi_stat.uid = stat.st_uid;
        abi_stat.gid = stat.st_gid;
        abi_stat.rdev = stat.st_rdev as u64;
        abi_stat.size = stat.st_size;
        abi_stat.blksize = stat.st_blksize as i32;
        abi_stat.blocks = stat.st_blocks;
        abi_stat.atime = stat.st_atime;
        abi_stat.atime_nsec = stat.st_atime_nsec;
        abi_stat.mtime = stat.st_mtime;
        abi_stat.mtime_nsec = stat.st_mtime_nsec;
        abi_stat.ctime = stat.st_ctime;
        abi_stat.ctime_nsec = stat.st_ctime_nsec;
        let len = size_of::<AbiStat>();
        let b: &[u8] = unsafe { from_raw_parts(&abi_stat as *const AbiStat as *const u8, len) };
        let addr = machine.registers()[A1].to_u64();
        machine.memory_mut().store_bytes(addr, b)?;
        machine.set_register(A0, Mac::REG::zero());
        Ok(())
    }

    fn lseek<Mac: SupportMachine>(&mut self, machine: &mut Mac) -> Result<(), Error> {
        let fd = machine.registers()[A0].to_i32();
        let offset = machine.registers()[A1].to_i64();
        let whence = match machine.registers()[A2].to_i32() {
            SEEK_SET => Whence::SeekSet,
            SEEK_CUR => Whence::SeekCur,
            SEEK_END => Whence::SeekEnd,
            _ => return Err(Error::Unexpected("Unexpected whence".into())),
        };
        let ret = lseek(fd, offset, whence).unwrap_or_else(|e| {
            debug!("lseek error: {:?}", e);
            -1
        });
        machine.set_register(A0, Mac::REG::from_i64(ret));
        Ok(())
    }

    fn read<Mac: SupportMachine>(&mut self, machine: &mut Mac) -> Result<(), Error> {
        let fd = machine.registers()[A0].to_i32();
        let addr = machine.registers()[A1].to_u64();
        let size = machine.registers()[A2].to_u64() as usize;
        let mut buf = vec![0u8; size];

        match read(fd, &mut buf) {
            Ok(read_size) => {
                machine.memory_mut().store_bytes(addr, &buf[0..read_size])?;
                machine.set_register(A0, Mac::REG::from_u64(read_size as u64));
            }
            Err(e) => {
                debug!("read error: {:?}", e);
                machine.set_register(A0, Mac::REG::from_i64(-1i64));
            }
        };
        Ok(())
    }

    fn write<Mac: SupportMachine>(&mut self, machine: &mut Mac) -> Result<(), Error> {
        let fd = machine.registers()[A0].to_i32();
        let addr = machine.registers()[A1].to_u64();
        let size = machine.registers()[A2].to_u64() as usize;
        let mut buf = vec![0u8; size];
        for i in 0..size {
            buf[i] = machine.memory_mut().load8(&Mac::REG::from_u64(addr + i as u64))?.to_u8();
        }
        let ret = write(fd, &buf).unwrap_or_else(|e| {
            debug!("write error: {:?}", e);
            (-1isize) as usize
        });
        machine.set_register(A0, Mac::REG::from_u64(ret as u64));
        Ok(())
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for Stdio {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        match machine.registers()[A7].to_u64() {
            57 => self.close(machine)?,
            62 => self.lseek(machine)?,
            63 => self.read(machine)?,
            64 => self.write(machine)?,
            80 => self.fstat(machine)?,
            _ => return Ok(false),
        };
        Ok(true)
    }
}
