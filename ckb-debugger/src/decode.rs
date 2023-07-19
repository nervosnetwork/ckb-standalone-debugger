use ckb_vm::ckb_vm_definitions::instructions::{Instruction, InstructionOpcode};

enum ISA {
    Imc,
    A,
    B,
    Unknow,
}

pub fn decode_instruction(data: &str) -> Result<(), Box<dyn std::error::Error>> {
    let instruction = if data.starts_with("0x") {
        u64::from_str_radix(&data[2..], 16)?
    } else {
        u64::from_str_radix(data, 10)?
    };

    let info = InstructionInfo::new(instruction);

    println!("instruction: {}", info.get_ins_info());
    println!("It is in RISC-V {}.", info.get_isa_str());
    Ok(())
}

struct InstructionInfo {
    instruction: Instruction,
    opcode: InstructionOpcode,
    isa: ISA,
}

impl InstructionInfo {
    pub fn new(instruction: Instruction) -> Self {
        let opcode = ckb_vm::instructions::extract_opcode(instruction);
        let isa = Self::get_isa(opcode);

        Self {
            instruction,
            opcode,
            isa,
        }
    }

    pub fn get_isa_str(&self) -> &str {
        match self.isa {
            ISA::Imc => "Base Instruction",
            ISA::A => "A extension",
            ISA::B => "B extension",
            ISA::Unknow => panic!("Unknow instruction: {}", self.opcode),
        }
    }

    pub fn get_ins_info(&self) -> String {
        use ckb_vm::instructions::*;

        let tagged_instruction_option = tagged::TaggedInstruction::try_from(self.instruction);
        if let Ok(tagged_instruction) = tagged_instruction_option {
            tagged_instruction.to_string()
        } else {
            panic!("unknown instruction")
        }
    }

    fn get_isa(ins_opcode: InstructionOpcode) -> ISA {
        use ckb_vm::ckb_vm_definitions::instructions::*;
        if (ins_opcode > OP_UNLOADED && ins_opcode <= OP_XORI)
            || (ins_opcode >= OP_WIDE_MUL && ins_opcode <= OP_CUSTOM_TRACE_END)
        {
            ISA::Imc
        } else if ins_opcode >= OP_LR_W && ins_opcode <= OP_AMOMAXU_D {
            ISA::A
        } else if ins_opcode >= OP_ADDUW && ins_opcode <= OP_ZEXTH {
            ISA::B
        } else {
            ISA::Unknow
        }
    }
}
