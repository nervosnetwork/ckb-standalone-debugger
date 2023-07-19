use ckb_vm::ckb_vm_definitions::instructions::{Instruction, InstructionOpcode};

enum ISA {
    Imc,
    A,
    B,
    Unknow,
}

pub fn decode_instruction(data: &str) -> Result<(), Box<dyn std::error::Error>> {
    let has_0x = data.find("0x");
    let instruction = if has_0x.is_some() && has_0x.unwrap() == 0 {
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
        use ckb_vm::ckb_vm_definitions::instructions::*;
        use ckb_vm::instructions::*;

        match self.opcode {
            OP_LB_VERSION0 | OP_LB_VERSION1 | OP_LH_VERSION0 | OP_LH_VERSION1 | OP_LW_VERSION0 | OP_LW_VERSION1
            | OP_LD_VERSION0 | OP_LD_VERSION1 | OP_LBU_VERSION0 | OP_LBU_VERSION1 | OP_LHU_VERSION0
            | OP_LHU_VERSION1 | OP_LWU_VERSION0 | OP_LWU_VERSION1 | OP_ADDI | OP_ADDIW | OP_XORI | OP_ORI | OP_ANDI
            | OP_SLTI | OP_SLTIU | OP_JALR_VERSION0 | OP_JALR_VERSION1 | OP_SLLI | OP_SRLI | OP_SRAI | OP_SLLIW
            | OP_SRLIW | OP_SRAIW | OP_BCLRI | OP_BEXTI | OP_BINVI | OP_BSETI | OP_RORI | OP_RORIW | OP_SLLIUW => {
                Itype(self.instruction).to_string()
            }
            OP_SB | OP_SH | OP_SW | OP_SD | OP_BEQ | OP_BNE | OP_BLT | OP_BGE | OP_BLTU | OP_BGEU => {
                Stype(self.instruction).to_string()
            }
            OP_LUI | OP_AUIPC | OP_JAL | OP_FAR_JUMP_REL | OP_FAR_JUMP_ABS | OP_CUSTOM_LOAD_UIMM
            | OP_CUSTOM_LOAD_IMM => Utype(self.instruction).to_string(),
            OP_ECALL => "ecall".to_string(),
            OP_EBREAK => "break".to_string(),
            OP_FENCEI => "fencei".to_string(),
            OP_FENCE => "fence".to_string(),
            OP_WIDE_MUL | OP_WIDE_MULU | OP_WIDE_MULSU | OP_WIDE_DIV | OP_WIDE_DIVU | OP_SBB | OP_ADCS | OP_SBBS => {
                R4type(self.instruction).to_string()
            }
            OP_ADD3A | OP_ADD3B | OP_ADD3C => R5type(self.instruction).to_string(),
            _ => Rtype(self.instruction).to_string(),
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
