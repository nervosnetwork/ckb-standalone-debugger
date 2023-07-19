pub fn decode_instruction(data: &str) -> Result<(), Box<dyn std::error::Error>> {
    let instruction = if data.starts_with("0x") {
        u32::from_str_radix(&data[2..], 16)?
    } else {
        u32::from_str_radix(data, 10)?
    };

    use ckb_vm::instructions::*;
    use ckb_vm::machine::VERSION2;

    let (ins, isa) = if let Some(i) = i::factory::<u64>(instruction, VERSION2) {
        let tagged_instruction: tagged::TaggedInstruction = tagged::TaggedInstruction::try_from(i).unwrap();
        (tagged_instruction.to_string(), "Integer Instruction")
    } else if let Some(i) = m::factory::<u64>(instruction, VERSION2) {
        let tagged_instruction = tagged::TaggedInstruction::try_from(i).unwrap();
        (tagged_instruction.to_string(), "M extension")
    } else if let Some(i) = a::factory::<u64>(instruction, VERSION2) {
        let tagged_instruction = tagged::TaggedInstruction::try_from(i).unwrap();
        (tagged_instruction.to_string(), "A extension")
    } else if let Some(i) = b::factory::<u64>(instruction, VERSION2) {
        let tagged_instruction = tagged::TaggedInstruction::try_from(i).unwrap();
        (tagged_instruction.to_string(), "B extension")
    } else if let Some(i) = rvc::factory::<u64>(instruction, VERSION2) {
        let tagged_instruction = tagged::TaggedInstruction::try_from(i).unwrap();
        (tagged_instruction.to_string(), "RVC")
    } else {
        panic!("unknow instruction")
    };

    println!("instruction: {}\nIt is in RISC-V {}.", ins, isa);

    Ok(())
}
