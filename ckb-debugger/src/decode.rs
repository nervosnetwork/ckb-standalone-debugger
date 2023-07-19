use ckb_vm::ckb_vm_definitions::instructions::{Instruction, InstructionOpcode};

enum ISA {
    Imc,
    A,
    B,
    Mop,
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
            ISA::Mop => "MOP extension",
            ISA::Unknow => panic!("Unknow instruction: {}", self.opcode),
        }
    }

    pub fn get_ins_info(&self) -> String {
        use ckb_vm::ckb_vm_definitions::instructions::*;
        use ckb_vm::instructions::*;

        match self.opcode {
            OP_SUB => Rtype(self.instruction).to_string(),
            OP_SUBW => Rtype(self.instruction).to_string(),
            OP_ADD => Rtype(self.instruction).to_string(),
            OP_ADDW => Rtype(self.instruction).to_string(),
            OP_XOR => Rtype(self.instruction).to_string(),
            OP_OR => Rtype(self.instruction).to_string(),
            OP_AND => Rtype(self.instruction).to_string(),
            OP_SLL => Rtype(self.instruction).to_string(),
            OP_SLLW => Rtype(self.instruction).to_string(),
            OP_SRL => Rtype(self.instruction).to_string(),
            OP_SRLW => Rtype(self.instruction).to_string(),
            OP_SRA => Rtype(self.instruction).to_string(),
            OP_SRAW => Rtype(self.instruction).to_string(),
            OP_SLT => Rtype(self.instruction).to_string(),
            OP_SLTU => Rtype(self.instruction).to_string(),
            OP_LB_VERSION0 => Itype(self.instruction).to_string(),
            OP_LB_VERSION1 => Itype(self.instruction).to_string(),
            OP_LH_VERSION0 => Itype(self.instruction).to_string(),
            OP_LH_VERSION1 => Itype(self.instruction).to_string(),
            OP_LW_VERSION0 => Itype(self.instruction).to_string(),
            OP_LW_VERSION1 => Itype(self.instruction).to_string(),
            OP_LD_VERSION0 => Itype(self.instruction).to_string(),
            OP_LD_VERSION1 => Itype(self.instruction).to_string(),
            OP_LBU_VERSION0 => Itype(self.instruction).to_string(),
            OP_LBU_VERSION1 => Itype(self.instruction).to_string(),
            OP_LHU_VERSION0 => Itype(self.instruction).to_string(),
            OP_LHU_VERSION1 => Itype(self.instruction).to_string(),
            OP_LWU_VERSION0 => Itype(self.instruction).to_string(),
            OP_LWU_VERSION1 => Itype(self.instruction).to_string(),
            OP_ADDI => Itype(self.instruction).to_string(),
            OP_ADDIW => Itype(self.instruction).to_string(),
            OP_XORI => Itype(self.instruction).to_string(),
            OP_LR_W => Rtype(self.instruction).to_string(),
            OP_SC_W => Rtype(self.instruction).to_string(),
            OP_AMOSWAP_W => Rtype(self.instruction).to_string(),
            OP_AMOADD_W => Rtype(self.instruction).to_string(),
            OP_AMOXOR_W => Rtype(self.instruction).to_string(),
            OP_AMOAND_W => Rtype(self.instruction).to_string(),
            OP_AMOOR_W => Rtype(self.instruction).to_string(),
            OP_AMOMIN_W => Rtype(self.instruction).to_string(),
            OP_AMOMAX_W => Rtype(self.instruction).to_string(),
            OP_AMOMINU_W => Rtype(self.instruction).to_string(),
            OP_AMOMAXU_W => Rtype(self.instruction).to_string(),
            OP_LR_D => Rtype(self.instruction).to_string(),
            OP_SC_D => Rtype(self.instruction).to_string(),
            OP_AMOSWAP_D => Rtype(self.instruction).to_string(),
            OP_AMOADD_D => Rtype(self.instruction).to_string(),
            OP_AMOXOR_D => Rtype(self.instruction).to_string(),
            OP_AMOAND_D => Rtype(self.instruction).to_string(),
            OP_AMOOR_D => Rtype(self.instruction).to_string(),
            OP_AMOMIN_D => Rtype(self.instruction).to_string(),
            OP_AMOMAX_D => Rtype(self.instruction).to_string(),
            OP_AMOMINU_D => Rtype(self.instruction).to_string(),
            OP_AMOMAXU_D => Rtype(self.instruction).to_string(),
            OP_ORI => Itype(self.instruction).to_string(),
            OP_ANDI => Itype(self.instruction).to_string(),
            OP_SLTI => Itype(self.instruction).to_string(),
            OP_SLTIU => Itype(self.instruction).to_string(),
            OP_JALR_VERSION0 => Itype(self.instruction).to_string(),
            OP_JALR_VERSION1 => Itype(self.instruction).to_string(),
            OP_SLLI => Itype(self.instruction).to_string(),
            OP_SRLI => Itype(self.instruction).to_string(),
            OP_SRAI => Itype(self.instruction).to_string(),
            OP_SLLIW => Itype(self.instruction).to_string(),
            OP_SRLIW => Itype(self.instruction).to_string(),
            OP_SRAIW => Itype(self.instruction).to_string(),
            OP_SB => Stype(self.instruction).to_string(),
            OP_SH => Stype(self.instruction).to_string(),
            OP_SW => Stype(self.instruction).to_string(),
            OP_SD => Stype(self.instruction).to_string(),
            OP_BEQ => Stype(self.instruction).to_string(),
            OP_BNE => Stype(self.instruction).to_string(),
            OP_BLT => Stype(self.instruction).to_string(),
            OP_BGE => Stype(self.instruction).to_string(),
            OP_BLTU => Stype(self.instruction).to_string(),
            OP_BGEU => Stype(self.instruction).to_string(),
            OP_LUI => Utype(self.instruction).to_string(),
            OP_AUIPC => Utype(self.instruction).to_string(),
            OP_ECALL => "ecall".to_string(),
            OP_EBREAK => "break".to_string(),
            OP_FENCEI => "fencei".to_string(),
            OP_FENCE => "fence".to_string(),
            OP_JAL => Utype(self.instruction).to_string(),
            OP_MUL => Rtype(self.instruction).to_string(),
            OP_MULW => Rtype(self.instruction).to_string(),
            OP_MULH => Rtype(self.instruction).to_string(),
            OP_MULHSU => Rtype(self.instruction).to_string(),
            OP_MULHU => Rtype(self.instruction).to_string(),
            OP_DIV => Rtype(self.instruction).to_string(),
            OP_DIVW => Rtype(self.instruction).to_string(),
            OP_DIVU => Rtype(self.instruction).to_string(),
            OP_DIVUW => Rtype(self.instruction).to_string(),
            OP_REM => Rtype(self.instruction).to_string(),
            OP_REMW => Rtype(self.instruction).to_string(),
            OP_REMU => Rtype(self.instruction).to_string(),
            OP_REMUW => Rtype(self.instruction).to_string(),
            OP_ADDUW => Rtype(self.instruction).to_string(),
            OP_ANDN => Rtype(self.instruction).to_string(),
            OP_BCLR => Rtype(self.instruction).to_string(),
            OP_BCLRI => Itype(self.instruction).to_string(),
            OP_BEXT => Rtype(self.instruction).to_string(),
            OP_BEXTI => Itype(self.instruction).to_string(),
            OP_BINV => Rtype(self.instruction).to_string(),
            OP_BINVI => Itype(self.instruction).to_string(),
            OP_BSET => Rtype(self.instruction).to_string(),
            OP_BSETI => Itype(self.instruction).to_string(),
            OP_CLMUL => Rtype(self.instruction).to_string(),
            OP_CLMULH => Rtype(self.instruction).to_string(),
            OP_CLMULR => Rtype(self.instruction).to_string(),
            OP_CLZ => Rtype(self.instruction).to_string(),
            OP_CLZW => Rtype(self.instruction).to_string(),
            OP_CPOP => Rtype(self.instruction).to_string(),
            OP_CPOPW => Rtype(self.instruction).to_string(),
            OP_CTZ => Rtype(self.instruction).to_string(),
            OP_CTZW => Rtype(self.instruction).to_string(),
            OP_MAX => Rtype(self.instruction).to_string(),
            OP_MAXU => Rtype(self.instruction).to_string(),
            OP_MIN => Rtype(self.instruction).to_string(),
            OP_MINU => Rtype(self.instruction).to_string(),
            OP_ORCB => Rtype(self.instruction).to_string(),
            OP_ORN => Rtype(self.instruction).to_string(),
            OP_REV8 => Rtype(self.instruction).to_string(),
            OP_ROL => Rtype(self.instruction).to_string(),
            OP_ROLW => Rtype(self.instruction).to_string(),
            OP_ROR => Rtype(self.instruction).to_string(),
            OP_RORI => Itype(self.instruction).to_string(),
            OP_RORIW => Itype(self.instruction).to_string(),
            OP_RORW => Rtype(self.instruction).to_string(),
            OP_SEXTB => Rtype(self.instruction).to_string(),
            OP_SEXTH => Rtype(self.instruction).to_string(),
            OP_SH1ADD => Rtype(self.instruction).to_string(),
            OP_SH1ADDUW => Rtype(self.instruction).to_string(),
            OP_SH2ADD => Rtype(self.instruction).to_string(),
            OP_SH2ADDUW => Rtype(self.instruction).to_string(),
            OP_SH3ADD => Rtype(self.instruction).to_string(),
            OP_SH3ADDUW => Rtype(self.instruction).to_string(),
            OP_SLLIUW => Itype(self.instruction).to_string(),
            OP_XNOR => Rtype(self.instruction).to_string(),
            OP_ZEXTH => Rtype(self.instruction).to_string(),
            OP_WIDE_MUL => R4type(self.instruction).to_string(),
            OP_WIDE_MULU => R4type(self.instruction).to_string(),
            OP_WIDE_MULSU => R4type(self.instruction).to_string(),
            OP_WIDE_DIV => R4type(self.instruction).to_string(),
            OP_WIDE_DIVU => R4type(self.instruction).to_string(),
            OP_FAR_JUMP_REL => Utype(self.instruction).to_string(),
            OP_FAR_JUMP_ABS => Utype(self.instruction).to_string(),
            OP_ADC => Rtype(self.instruction).to_string(),
            OP_SBB => R4type(self.instruction).to_string(),
            OP_ADCS => R4type(self.instruction).to_string(),
            OP_SBBS => R4type(self.instruction).to_string(),
            OP_ADD3A => R5type(self.instruction).to_string(),
            OP_ADD3B => R5type(self.instruction).to_string(),
            OP_ADD3C => R5type(self.instruction).to_string(),
            OP_CUSTOM_LOAD_UIMM => Utype(self.instruction).to_string(),
            OP_CUSTOM_LOAD_IMM => Utype(self.instruction).to_string(),
            _ => panic!("unknow instruction opcode"),
        }
    }

    fn get_isa(ins_opcode: InstructionOpcode) -> ISA {
        use ckb_vm::ckb_vm_definitions::instructions::*;
        if ins_opcode > OP_UNLOADED && ins_opcode <= OP_XORI {
            ISA::Imc
        } else if ins_opcode >= OP_LR_W && ins_opcode <= OP_AMOMAXU_D {
            ISA::A
        } else if ins_opcode >= OP_ADDUW && ins_opcode <= OP_ZEXTH {
            ISA::B
        } else if ins_opcode >= OP_WIDE_MUL && ins_opcode <= OP_CUSTOM_TRACE_END {
            ISA::Mop
        } else {
            ISA::Unknow
        }
    }
}
