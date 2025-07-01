use crate::arch;
use ckb_vm::machine::VERSION2;

/// Print information about an instruction to assist humans in analysis. Get inspired by
/// https://luplab.gitlab.io/rvcodecjs/.
pub fn instruction_decode(inst: u32) {
    let mut inst_tag = String::from("?");
    let mut inst_isa = String::from("?");
    if let Some(i) = ckb_vm::instructions::i::factory::<u64>(inst, VERSION2) {
        assert_eq!(inst_tag.as_str(), "?");
        let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
        inst_tag = tagged_instruction.to_string();
        inst_isa = "I".to_string();
    }
    if let Some(i) = ckb_vm::instructions::m::factory::<u64>(inst, VERSION2) {
        assert_eq!(inst_tag.as_str(), "?");
        let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
        inst_tag = tagged_instruction.to_string();
        inst_isa = "M".to_string();
    }
    if let Some(i) = ckb_vm::instructions::a::factory::<u64>(inst, VERSION2) {
        assert_eq!(inst_tag.as_str(), "?");
        let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
        inst_tag = tagged_instruction.to_string();
        inst_isa = "A".to_string();
    }
    if let Some(i) = ckb_vm::instructions::rvc::factory::<u64>(inst, VERSION2) {
        assert_eq!(inst_tag.as_str(), "?");
        let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
        inst_tag = tagged_instruction.to_string();
        inst_isa = "C".to_string();
    }
    if let Some(i) = ckb_vm::instructions::b::factory::<u64>(inst, VERSION2) {
        assert_eq!(inst_tag.as_str(), "?");
        let tagged_instruction = ckb_vm::instructions::tagged::TaggedInstruction::try_from(i).unwrap();
        inst_tag = tagged_instruction.to_string();
        inst_isa = "B".to_string();
    }
    arch::println(&format!("       Assembly = {}", inst_tag));
    if inst_isa == "C" {
        arch::println(&format!("         Binary = {:016b}", inst));
        arch::println(&format!("    Hexadecimal = {:04x}", inst));
    } else {
        arch::println(&format!("         Binary = {:032b}", inst));
        arch::println(&format!("    Hexadecimal = {:08x}", inst));
    }
    arch::println(&format!("Instruction set = {}", inst_isa));
}
