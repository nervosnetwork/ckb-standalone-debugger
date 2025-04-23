use ckb_vm::{
    Error, Memory, Register, SupportMachine, Syscalls,
    registers::{A0, A1, A2, A7},
};

#[derive(Clone, Default)]
pub struct FileWriter {}

impl FileWriter {
    pub fn new() -> Self {
        FileWriter {}
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for FileWriter {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        let id = machine.registers()[A7].to_u64();
        if id != 9013 {
            return Ok(false);
        }
        let path_ptr = machine.registers()[A0].clone();
        let path = ckb_vm::memory::load_c_string_byte_by_byte(machine.memory_mut(), &path_ptr)?;
        let addr = machine.registers()[A1].to_u64();
        let size = machine.registers()[A2].to_u64();
        let data = machine.memory_mut().load_bytes(addr, size)?;
        if let Ok(_) = crate::arch::file_write(&String::from_utf8_lossy(&path), &data) {
            machine.set_register(A0, Mac::REG::from_u64(0));
        } else {
            machine.set_register(A0, Mac::REG::from_i64(-1));
        }
        return Ok(true);
    }
}
