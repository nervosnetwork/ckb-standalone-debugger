use ckb_vm::{
    Error, Register, SupportMachine, Syscalls,
    registers::{A0, A7},
};

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
        if id != 9002 {
            return Ok(false);
        }
        let r = crate::arch::random();
        machine.set_register(A0, Mac::REG::from_u64(r));
        return Ok(true);
    }
}
