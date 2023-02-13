use byteorder::{ByteOrder, LittleEndian};
use bytes::{BufMut, Bytes, BytesMut};
use ckb_vm::{
    memory::{FLAG_EXECUTABLE, FLAG_WXORX_BIT},
    registers::A7,
    Error, Memory, Register, SupportMachine, Syscalls, RISCV_PAGES, RISCV_PAGESIZE,
};
use std::fs::File;
use std::io::Write;

pub struct ElfDumper {
    dump_file_name: String,
    syscall_number: u64,
    maximum_zero_gap: u64,
}

impl Default for ElfDumper {
    fn default() -> ElfDumper {
        ElfDumper {
            dump_file_name: "dump.bin".to_string(),
            syscall_number: 4097,
            maximum_zero_gap: 64,
        }
    }
}

impl ElfDumper {
    pub fn new(dump_file_name: String, syscall_number: u64, maximum_zero_gap: u64) -> Self {
        ElfDumper {
            dump_file_name,
            syscall_number,
            maximum_zero_gap,
        }
    }
}

#[derive(Clone)]
struct Segment {
    start: u64,
    data: Bytes,
    executable: bool,
}

impl Segment {
    fn first_page(&self) -> u64 {
        self.start / RISCV_PAGESIZE as u64
    }

    fn first_page_address(&self) -> u64 {
        self.first_page() * RISCV_PAGESIZE as u64
    }

    fn last_page(&self) -> u64 {
        (self.start + self.data.len() as u64 - 1) / RISCV_PAGESIZE as u64
    }
}

impl<Mac: SupportMachine> Syscalls<Mac> for ElfDumper {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, Error> {
        if machine.registers()[A7].to_u64() != self.syscall_number {
            return Ok(false);
        }
        let mut segments: Vec<Segment> = vec![];
        let mut page = 0;
        // Extract all non-empty data from memory
        while page < RISCV_PAGES as u64 {
            let mut start = page * RISCV_PAGESIZE as u64;
            let end = (page + 1) * RISCV_PAGESIZE as u64;

            while start < end {
                // First, loop for the start of non-zero values
                while start < end {
                    if machine.memory_mut().load64(&Mac::REG::from_u64(start))?.to_u64() != 0 {
                        break;
                    }
                    start += 8;
                }

                if start < end {
                    // See if we can append to last segment
                    let executable = machine.memory_mut().fetch_flag(page)? & FLAG_WXORX_BIT == FLAG_EXECUTABLE;
                    let (bytes_start, mut bytes_mut) = if segments.is_empty() {
                        (start, BytesMut::new())
                    } else {
                        let last_segment = &segments[segments.len() - 1];
                        let same_page = page == last_segment.last_page();
                        let gap = start - (last_segment.start + last_segment.data.len() as u64);
                        if last_segment.executable == executable && ((gap <= self.maximum_zero_gap) || same_page) {
                            let Segment {
                                start: segment_start,
                                data: segment_data,
                                ..
                            } = segments.remove(segments.len() - 1);
                            let mut segment_data = BytesMut::from(segment_data.as_ref());
                            // Fill in gap first
                            let mut zeros = vec![];
                            zeros.resize(gap as usize, 0);
                            segment_data.extend_from_slice(&zeros);
                            (segment_start, segment_data)
                        } else {
                            (start, BytesMut::new())
                        }
                    };

                    // Append non-zero data
                    while start < end {
                        let value = machine.memory_mut().load64(&Mac::REG::from_u64(start))?.to_u64();
                        if value == 0 {
                            break;
                        }

                        bytes_mut.put_u64_le(value);
                        start += 8;
                    }

                    segments.push(Segment {
                        start: bytes_start,
                        data: bytes_mut.freeze(),
                        executable,
                    });
                }
            }
            page += 1;
        }
        // There must be one page at least before the first segment, so we can
        // allocate code we need.
        if segments.is_empty() || segments[0].start <= RISCV_PAGESIZE as u64 {
            return Err(Error::Unexpected("Unexpected segments".into()));
        }

        // Build instructions that restore register values
        let mut register_buffer = BytesMut::new();
        for register_value in &machine.registers()[1..] {
            register_buffer.put_u64_le(register_value.to_u64());
        }
        let register_entrypoint = register_buffer.len() as u64;
        register_buffer.put_u32_le(0x00000517); // auipc a0, 0x0
        register_buffer.put_u32_le(0xf0050513); // addi a0, a0, -256
        register_buffer.put_u32_le(0x00853083); // ld ra,8(a0)
        register_buffer.put_u32_le(0x01053103); // ld sp,16(a0)
        register_buffer.put_u32_le(0x01853183); // ld gp,24(a0)
        register_buffer.put_u32_le(0x02053203); // ld tp,32(a0)
        register_buffer.put_u32_le(0x02853283); // ld t0,40(a0)
        register_buffer.put_u32_le(0x03053303); // ld t1,48(a0)
        register_buffer.put_u32_le(0x03853383); // ld t2,56(a0)
        register_buffer.put_u32_le(0x04053403); // ld s0,64(a0)
        register_buffer.put_u32_le(0x04853483); // ld s1,72(a0)
        register_buffer.put_u32_le(0x05853583); // ld a1,88(a0)
        register_buffer.put_u32_le(0x06053603); // ld a2,96(a0)
        register_buffer.put_u32_le(0x06853683); // ld a3,104(a0)
        register_buffer.put_u32_le(0x07053703); // ld a4,112(a0)
        register_buffer.put_u32_le(0x07853783); // ld a5,120(a0)
        register_buffer.put_u32_le(0x08053803); // ld a6,128(a0)
        register_buffer.put_u32_le(0x08853883); // ld a7,136(a0)
        register_buffer.put_u32_le(0x09053903); // ld s2,144(a0)
        register_buffer.put_u32_le(0x09853983); // ld s3,152(a0)
        register_buffer.put_u32_le(0x0a053a03); // ld s4,160(a0)
        register_buffer.put_u32_le(0x0a853a83); // ld s5,168(a0)
        register_buffer.put_u32_le(0x0b053b03); // ld s6,176(a0)
        register_buffer.put_u32_le(0x0b853b83); // ld s7,184(a0)
        register_buffer.put_u32_le(0x0c053c03); // ld s8,192(a0)
        register_buffer.put_u32_le(0x0c853c83); // ld s9,200(a0)
        register_buffer.put_u32_le(0x0d053d03); // ld s10,208(a0)
        register_buffer.put_u32_le(0x0d853d83); // ld s11,216(a0)
        register_buffer.put_u32_le(0x0e053e03); // ld t3,224(a0)
        register_buffer.put_u32_le(0x0e853e83); // ld t4,232(a0)
        register_buffer.put_u32_le(0x0f053f03); // ld t5,240(a0)
        register_buffer.put_u32_le(0x0f853f83); // ld t6,248(a0)
        register_buffer.put_u32_le(0x05053503); // ld a0,80(a0)

        let register_buffer_start = segments[0].first_page_address() - RISCV_PAGESIZE as u64;
        let jump_instruction_pc = register_buffer_start + register_buffer.len() as u64;
        let jump_offset = machine.pc().to_u64() - jump_instruction_pc;
        let masked = jump_offset & 0xFFFFFFFFFFE00001;
        if masked != 0 && masked != 0xFFFFFFFFFFE00000 {
            return Err(Error::Unexpected("Unexpected masked".into()));
        }
        let jump_instruction = 0b1101111
            | ((((jump_offset >> 12) & 0b_1111_1111) as u32) << 12)
            | ((((jump_offset >> 11) & 1) as u32) << 20)
            | ((((jump_offset >> 1) & 0b_1111_1111_11) as u32) << 21)
            | ((((jump_offset >> 20) & 1) as u32) << 31);
        register_buffer.put_u32_le(jump_instruction);
        assert!(register_buffer.len() < RISCV_PAGESIZE);

        segments.push(Segment {
            start: register_buffer_start,
            data: register_buffer.freeze(),
            executable: true,
        });

        // Piece everything together into an ELF binary
        let mut elf = BytesMut::new();
        // ELF Magic
        elf.extend_from_slice(&[
            0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        // ELF Type
        elf.put_u16_le(2);
        // ELF Machine; RISC-V
        elf.put_u16_le(243);
        // ELF Version
        elf.put_u32_le(1);
        // ELF Entry
        elf.put_u64_le(register_buffer_start + register_entrypoint);
        let program_header_offset = elf.len();
        // Program Header Offset, placeholder for now
        elf.put_u64_le(0);
        let section_header_offset = elf.len();
        // Section Header Offset, placeholder for now
        elf.put_u64_le(0);
        // ELF Flags: 0x1, RVC, soft-float ABI
        elf.put_u32_le(1);
        // ELF Header Size: 64
        elf.put_u16_le(64);
        // Program Header Entry Size: 56
        elf.put_u16_le(56);
        let program_header_number_offset = elf.len();
        // Program Header Number, placeholder for now
        elf.put_u16_le(0);
        // Section Header Entry Size: 64
        elf.put_u16_le(64);
        let section_header_number_offset = elf.len();
        // Section Header Number, placeholder for now
        elf.put_u16_le(0);
        // Section header string table index: 0
        elf.put_u16_le(0);
        assert!(elf.len() == 64);

        let string_table_offset = elf.len() as u64;
        elf.put_u32_le(0);

        let mut section_headers = vec![];
        let mut string_table_section_header = BytesMut::new();
        // Name
        string_table_section_header.put_u32_le(0);
        // Type: STRTAB
        string_table_section_header.put_u32_le(3);
        // Flags
        string_table_section_header.put_u64_le(0);
        // Address
        string_table_section_header.put_u64_le(0);
        // Offset
        string_table_section_header.put_u64_le(string_table_offset);
        // Size
        string_table_section_header.put_u64_le(4);
        // Link
        string_table_section_header.put_u32_le(0);
        // Info
        string_table_section_header.put_u32_le(0);
        // Align
        string_table_section_header.put_u64_le(1);
        // Entry size
        string_table_section_header.put_u64_le(0);
        assert!(string_table_section_header.len() == 64);
        section_headers.push(string_table_section_header.freeze());

        let mut program_headers = vec![];

        for segment in segments {
            let current_offset = elf.len() as u64;
            elf.extend_from_slice(segment.data.as_ref());

            let mut program_header = BytesMut::new();
            // Type: LOAD
            program_header.put_u32_le(1);
            // Flags
            program_header.put_u32_le(if segment.executable { 5 } else { 6 });
            // Offset
            program_header.put_u64_le(current_offset);
            // Vaddr
            program_header.put_u64_le(segment.start);
            // Paddr
            program_header.put_u64_le(segment.start);
            // File size
            program_header.put_u64_le(segment.data.len() as u64);
            // Memory size
            program_header.put_u64_le(segment.data.len() as u64);
            // Align
            program_header.put_u64_le(0x1000);
            assert!(program_header.len() == 56);
            program_headers.push(program_header.freeze());

            // TODO: add an option to provide a binary, and use the binary's
            // section headers to replace the inferred ones here. Since inferred
            // sections here can contain non-code data section as well.
            if segment.executable {
                let mut section_header = BytesMut::new();
                // Name
                section_header.put_u32_le(0);
                // Type: PROGBITS
                section_header.put_u32_le(1);
                // Flags: AX
                section_header.put_u64_le(6);
                // Address
                section_header.put_u64_le(segment.start);
                // Offset
                section_header.put_u64_le(current_offset);
                // Size
                section_header.put_u64_le(segment.data.len() as u64);
                // Link
                section_header.put_u32_le(0);
                // Info
                section_header.put_u32_le(0);
                // Align
                section_header.put_u64_le(2);
                // Entry size
                section_header.put_u64_le(0);
                assert!(section_header.len() == 64);
                section_headers.push(section_header.freeze());
            }
        }

        while elf.len() % 4 != 0 {
            elf.put_u8(0);
        }
        let current_offset = elf.len() as u64;
        LittleEndian::write_u64(
            &mut elf[program_header_offset..program_header_offset + 8],
            current_offset,
        );
        LittleEndian::write_u16(
            &mut elf[program_header_number_offset..program_header_number_offset + 8],
            program_headers.len() as u16,
        );
        for program_header in program_headers {
            elf.extend_from_slice(program_header.as_ref());
        }

        while elf.len() % 4 != 0 {
            elf.put_u8(0);
        }
        let current_offset = elf.len() as u64;
        LittleEndian::write_u64(
            &mut elf[section_header_offset..section_header_offset + 8],
            current_offset,
        );
        LittleEndian::write_u16(
            &mut elf[section_header_number_offset..section_header_number_offset + 8],
            section_headers.len() as u16,
        );
        for section_header in section_headers {
            elf.extend_from_slice(section_header.as_ref());
        }

        let mut file = File::create(&self.dump_file_name)?;
        file.write_all(&elf)?;

        Ok(true)
    }
}
