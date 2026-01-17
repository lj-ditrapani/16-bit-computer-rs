use crate::error::MemoryError;
use crate::memory::Memory;

pub struct Cpu {
    // General purpose registers R0-R15
    pub registers: [u16; 16],

    // Program counter (16-bit)
    pub pc: u16,

    // Status flags
    pub carry: bool,    // C flag
    pub overflow: bool, // V flag
}

#[derive(Debug)]
pub enum InstructionResult {
    Jump(u16), // Set pc to address
    Next,      // Increment pc
    Halt,      // END instruction executed
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: [0u16; 16],
            pc: 0,
            carry: false,
            overflow: false,
        }
    }

    pub fn step(&mut self, memory: &mut Memory) -> Result<InstructionResult, MemoryError> {
        let instruction = memory.read_program(self.pc)?;
        let instruction_result = self.execute_instruction(instruction, memory)?;
        let pc = match instruction_result {
            InstructionResult::Jump(addr) => addr,
            InstructionResult::Next => self.pc.wrapping_add(1),
            InstructionResult::Halt => self.pc,
        };
        self.pc = pc;
        Ok(instruction_result)
    }

    fn execute_instruction(
        &mut self,
        instruction: u16,
        memory: &mut Memory,
    ) -> Result<InstructionResult, MemoryError> {
        let opcode = (instruction >> 12) as u8 & 0x0F;
        let rs1 = ((instruction >> 8) & 0xF) as u8;
        let rs2 = ((instruction >> 4) & 0xF) as u8;
        let rd = (instruction & 0xF) as u8;

        match opcode {
            0x0 => Ok(self.end()),
            0x1 => Ok(self.hby(instruction, rd)),
            0x2 => Ok(self.lby(instruction, rd)),
            0x3 => Ok(self.lod(rs1, rd, memory)),
            0x4 => self.str(rs1, rs2, memory),
            0x5 => Ok(self.add(rs1, rs2, rd)),
            0x6 => Ok(self.sub(rs1, rs2, rd)),
            0x7 => Ok(self.adi(rs1, rs2, rd)),
            0x8 => Ok(self.sbi(rs1, rs2, rd)),
            0x9 => Ok(self.and(rs1, rs2, rd)),
            0xA => Ok(self.orr(rs1, rs2, rd)),
            0xB => Ok(self.xor(rs1, rs2, rd)),
            0xC => Ok(self.nor(rs1, rs2, rd)),
            0xD => Ok(self.shf(rs1, rs2, rd)),
            0xE => Ok(self.brv(rs1, rs2, rd)),
            _ => Ok(self.brf(rs2, rd)), // _ can only be 0xF since opcode is masked to 4 bits
        }
    }

    // Instruction implementations

    fn end(&self) -> InstructionResult {
        InstructionResult::Halt
    }

    fn hby(&mut self, instruction: u16, rd: u8) -> InstructionResult {
        // HBY: immd8 -> RD[15-08]
        // RS1+RS2 form 8-bit immediate
        let immd8 = (instruction << 4) & 0xFF00;
        self.set_register(rd, (self.register(rd) & 0x00FF) | immd8);
        InstructionResult::Next
    }

    fn lby(&mut self, instruction: u16, rd: u8) -> InstructionResult {
        // LBY: immd8 -> RD[07-00]
        // RS1+RS2 form 8-bit immediate
        let immd8 = (instruction >> 4) & 0x00FF;
        self.set_register(rd, (self.register(rd) & 0xFF00) | immd8);
        InstructionResult::Next
    }

    fn lod(&mut self, rs1: u8, rd: u8, memory: &Memory) -> InstructionResult {
        // LOD: ram[RS1] -> RD
        let address = self.register(rs1);
        let value = memory.read_data(address);
        self.set_register(rd, value);
        InstructionResult::Next
    }

    fn str(&self, rs1: u8, rs2: u8, memory: &mut Memory) -> Result<InstructionResult, MemoryError> {
        // STR: RS2 -> ram[RS1]
        let address = self.register(rs1);
        let value = self.register(rs2);
        memory.write_data(address, value)?;
        Ok(InstructionResult::Next)
    }

    fn add(&mut self, rs1: u8, rs2: u8, rd: u8) -> InstructionResult {
        // ADD: RS1 + RS2 -> RD
        let a = self.register(rs1);
        let b = self.register(rs2);
        self.basic_add(a, b, rd)
    }

    fn sub(&mut self, rs1: u8, rs2: u8, rd: u8) -> InstructionResult {
        // SUB: RS1 - RS2 -> RD
        let a = self.register(rs1);
        let b = self.register(rs2);
        self.basic_add(a, (!b) + 1, rd)
    }

    fn adi(&mut self, rs1: u8, immd4: u8, rd: u8) -> InstructionResult {
        // ADI: RS1 + immd4 -> RD
        let a = self.register(rs1);
        self.basic_add(a, immd4 as u16, rd)
    }

    fn sbi(&mut self, rs1: u8, immd4: u8, rd: u8) -> InstructionResult {
        // SBI: RS1 - immd4 -> RD
        let a = self.register(rs1);
        let immd = immd4 as u16;
        self.basic_add(a, (!immd) + 1, rd)
    }

    fn and(&mut self, rs1: u8, rs2: u8, rd: u8) -> InstructionResult {
        // AND: RS1 and RS2 -> RD
        self.set_register(rd, self.register(rs1) & self.register(rs2));
        InstructionResult::Next
    }

    fn orr(&mut self, rs1: u8, rs2: u8, rd: u8) -> InstructionResult {
        // ORR: RS1 or RS2 -> RD
        self.set_register(rd, self.register(rs1) | self.register(rs2));
        InstructionResult::Next
    }

    fn xor(&mut self, rs1: u8, rs2: u8, rd: u8) -> InstructionResult {
        // XOR: RS1 xor RS2 -> RD
        self.set_register(rd, self.register(rs1) ^ self.register(rs2));
        InstructionResult::Next
    }

    fn nor(&mut self, rs1: u8, rs2: u8, rd: u8) -> InstructionResult {
        // NOR: RS1 nor RS2 -> RD
        self.set_register(rd, !(self.register(rs1) | self.register(rs2)));
        InstructionResult::Next
    }

    fn shf(&mut self, rs1: u8, da: u8, rd: u8) -> InstructionResult {
        // SHF: RS1 shifted by immd4 -> RD
        // immd4 format: DAAA
        // D is direction: 0 left, 1 right
        // AAA is (amount - 1), so 0-7 -> 1-8
        let direction = da >> 3;
        let amount = (da & 0x7) + 1;

        let value = self.register(rs1);
        let result = if direction == 0 {
            // Left shift
            let shifted = value << amount;
            self.carry = (value >> (16 - amount)) & 1 != 0;
            shifted
        } else {
            // Right shift
            let shifted = value >> amount;
            self.carry = (value >> (amount - 1)) & 1 != 0;
            shifted
        };

        self.set_register(rd, result);
        InstructionResult::Next
    }

    fn brv(&self, rs1: u8, rs2: u8, cond: u8) -> InstructionResult {
        // BRV: if (RS1 matches NZP) then (RS2 -> PC)
        // RD contains condition bits: 0NZP
        let value = self.register(rs1);

        // Check sign of value
        let is_negative = (value & 0x8000) != 0;
        let is_zero = value == 0;
        let is_positive = !is_negative && !is_zero;

        // Condition bits: 0NZP (bit 3 unused, bit 2=negative, bit 1=zero, bit 0=positive)
        let should_jump = is_bit_set(cond, 0b0100) && is_negative
            || is_bit_set(cond, 0b0010) && is_zero
            || is_bit_set(cond, 0b0001) && is_positive;

        if should_jump {
            InstructionResult::Jump(self.register(rs2))
        } else {
            InstructionResult::Next
        }
    }

    fn brf(&self, rs2: u8, cond: u8) -> InstructionResult {
        // BRF: if (C or V is set) then (RS2 -> PC)
        // RD contains condition bits: 00VC
        // Condition bits: 00VC (bits 3-2 unused, bit 1=overflow, bit 0=carry)
        let should_jump: bool = cond == 0 && !self.carry && !self.overflow
            || is_bit_set(cond, 0b0001) && self.carry
            || is_bit_set(cond, 0b0010) && self.overflow;

        if should_jump {
            InstructionResult::Jump(self.register(rs2))
        } else {
            InstructionResult::Next
        }
    }

    fn basic_add(&mut self, a: u16, b: u16, rd: u8) -> InstructionResult {
        let (result, carry) = a.carrying_add(b, false);

        self.set_register(rd, result);
        self.carry = carry;
        self.overflow = ((a ^ b) & 0x8000) == 0 && ((a ^ result) & 0x8000) != 0;

        InstructionResult::Next
    }

    fn register(&self, index: u8) -> u16 {
        self.registers[index as usize]
    }

    fn set_register(&mut self, index: u8, value: u16) {
        self.registers[index as usize] = value
    }

    pub fn dump_state(&self) {
        println!("CPU state:");
        println!("  PC: ${:04X}", self.pc);
        println!("  Carry: {}", self.carry);
        println!("  Overflow: {}", self.overflow);
        println!("  Registers:");
        for (i, reg) in self.registers.iter().enumerate() {
            println!("    R{}: ${:04X} ({})", i, reg, reg);
        }
    }
}

fn is_bit_set(cond: u8, mask: u8) -> bool {
    (cond & mask) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a memory instance for testing
    fn create_test_memory() -> Memory {
        Memory {
            program_rom: [0u16; 65536],
            data_rom: [0u16; 32768],
            ram: [0u16; 32768],
        }
    }

    // Helper function to build an instruction
    // Format: opcode (4 bits) | rs1 (4 bits) | rs2 (4 bits) | rd (4 bits)
    fn build_instruction(opcode: u8, rs1: u8, rs2: u8, rd: u8) -> u16 {
        ((opcode as u16) << 12) | ((rs1 as u16) << 8) | ((rs2 as u16) << 4) | (rd as u16)
    }

    #[test]
    fn test_end() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // Set up END instruction at address 0
        memory.program_rom[0] = 0x0000; // END instruction

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Halt));
        assert_eq!(cpu.pc, 0); // PC should not advance on HALT
    }

    #[test]
    fn test_hby() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // HBY: immd8 -> RD[15-08]
        // Instruction format: opcode=0x1, rs1=0xA, rs2=0xB, rd=0x5
        // immd8 is formed from rs1+rs2: bits 11-4 of instruction = 0xAB
        // So immd8 = 0xAB, which should go to bits 15-8 of RD
        cpu.registers[5] = 0x00CD; // Set lower byte

        // Build instruction: opcode=1, rs1=0xA, rs2=0xB, rd=5
        // immd8 = (instruction << 4) & 0xFF00 = bits 11-4 = 0xAB00
        memory.program_rom[0] = build_instruction(0x1, 0xA, 0xB, 0x5);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[5], 0xABCD); // Upper byte set, lower byte preserved
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_hby_zero() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // HBY with zero immediate
        cpu.registers[3] = 0x00FF;

        memory.program_rom[0] = build_instruction(0x1, 0x0, 0x0, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0x00FF); // Upper byte cleared, lower byte preserved
    }

    #[test]
    fn test_hby_max() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // HBY with maximum immediate (0xFF)
        cpu.registers[7] = 0x0012;

        memory.program_rom[0] = build_instruction(0x1, 0xF, 0xF, 0x7);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[7], 0xFF12); // Upper byte set to 0xFF
    }

    #[test]
    fn test_lby() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // LBY: immd8 -> RD[07-00]
        // Instruction format: opcode=0x2, rs1=0xC, rs2=0xD, rd=0x3
        // immd8 is formed from rs1+rs2: bits 11-4 of instruction = 0xCD
        cpu.registers[3] = 0xAB00; // Set upper byte

        // Build instruction: opcode=2, rs1=0xC, rs2=0xD, rd=3
        // immd8 = (instruction >> 4) & 0x00FF = bits 11-4 = 0xCD
        memory.program_rom[0] = build_instruction(0x2, 0xC, 0xD, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0xABCD); // Lower byte set, upper byte preserved
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_lby_zero() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // LBY with zero immediate
        cpu.registers[2] = 0xFF00;

        memory.program_rom[0] = build_instruction(0x2, 0x0, 0x0, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0xFF00); // Lower byte cleared, upper byte preserved
    }

    #[test]
    fn test_lby_max() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // LBY with maximum immediate (0xFF)
        cpu.registers[4] = 0x1200;

        memory.program_rom[0] = build_instruction(0x2, 0xF, 0xF, 0x4);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[4], 0x12FF); // Lower byte set to 0xFF
    }

    #[test]
    fn test_lod() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // LOD: ram[RS1] -> RD
        cpu.registers[2] = 0x8005; // Address in RAM
        memory.ram[5] = 0x1234; // Value at address 0x8005

        memory.program_rom[0] = build_instruction(0x3, 0x2, 0x0, 0x7); // LOD R2 -> R7

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[7], 0x1234);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_lod_data_rom() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // LOD from data ROM
        cpu.registers[1] = 0x0005; // Address in data ROM
        memory.data_rom[5] = 0x5678; // Value at address 0x0005

        memory.program_rom[0] = build_instruction(0x3, 0x1, 0x0, 0x8); // LOD R1 -> R8

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[8], 0x5678);
    }

    #[test]
    fn test_str() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // STR: RS2 -> ram[RS1]
        cpu.registers[3] = 0x8010; // Address in RAM
        cpu.registers[4] = 0xABCD; // Value to store

        memory.program_rom[0] = build_instruction(0x4, 0x3, 0x4, 0x0); // STR R4 -> ram[R3]

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(memory.ram[0x10], 0xABCD);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_str_readonly_error() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // STR to read-only data ROM should fail
        cpu.registers[1] = 0x0005; // Address in data ROM (read-only)
        cpu.registers[2] = 0x1234;

        memory.program_rom[0] = build_instruction(0x4, 0x1, 0x2, 0x0);

        let result = cpu.step(&mut memory);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MemoryError::ReadOnly(0x0005)));
    }

    #[test]
    fn test_add() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // ADD: RS1 + RS2 -> RD
        cpu.registers[1] = 0x1234;
        cpu.registers[2] = 0x5678;

        memory.program_rom[0] = build_instruction(0x5, 0x1, 0x2, 0x3); // ADD R1, R2 -> R3

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0x68AC);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_add_with_carry() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // ADD with carry
        cpu.registers[5] = 0xFFFF;
        cpu.registers[6] = 0x0001;

        memory.program_rom[0] = build_instruction(0x5, 0x5, 0x6, 0x7);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[7], 0x0000); // Wraps around
        assert!(cpu.carry); // Carry flag should be set
    }

    #[test]
    fn test_add_with_overflow() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // ADD with overflow (positive + positive = negative)
        cpu.registers[1] = 0x7FFF; // Max positive
        cpu.registers[2] = 0x0001;

        memory.program_rom[0] = build_instruction(0x5, 0x1, 0x2, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0x8000); // Overflow to negative
        assert!(cpu.overflow); // Overflow flag should be set
    }

    #[test]
    fn test_sub() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SUB: RS1 - RS2 -> RD
        cpu.registers[1] = 0x5678;
        cpu.registers[2] = 0x1234;

        memory.program_rom[0] = build_instruction(0x6, 0x1, 0x2, 0x3); // SUB R1, R2 -> R3

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0x4444);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_sub_zero() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SUB with same values (result should be zero)
        cpu.registers[1] = 0x1234;
        cpu.registers[2] = 0x1234;

        memory.program_rom[0] = build_instruction(0x6, 0x1, 0x2, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0x0000);
    }

    #[test]
    fn test_sub_with_borrow() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SUB with borrow (result wraps around)
        cpu.registers[1] = 0x0001;
        cpu.registers[2] = 0x0002;

        memory.program_rom[0] = build_instruction(0x6, 0x1, 0x2, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0xFFFF); // Wraps around (0x0001 - 0x0002 = 0xFFFF)
    }

    #[test]
    fn test_adi() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // ADI: RS1 + immd4 -> RD
        // immd4 is in rs2 field (4 bits)
        cpu.registers[1] = 0x1000;

        // ADI R1, 0x5 -> R2 (immd4 = 5)
        memory.program_rom[0] = build_instruction(0x7, 0x1, 0x5, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x1005);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_adi_max_immediate() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // ADI with maximum immediate (0xF = 15)
        cpu.registers[1] = 0x1000;

        memory.program_rom[0] = build_instruction(0x7, 0x1, 0xF, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x100F);
    }

    #[test]
    fn test_sbi() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SBI: RS1 - immd4 -> RD
        cpu.registers[1] = 0x1005;

        // SBI R1, 0x3 -> R2 (immd4 = 3)
        memory.program_rom[0] = build_instruction(0x8, 0x1, 0x3, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x1002);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_sbi_max_immediate() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SBI with maximum immediate (0xF = 15)
        cpu.registers[1] = 0x100F;

        memory.program_rom[0] = build_instruction(0x8, 0x1, 0xF, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x1000);
    }

    #[test]
    fn test_and() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // AND: RS1 and RS2 -> RD
        cpu.registers[1] = 0xFF00;
        cpu.registers[2] = 0x0F0F;

        memory.program_rom[0] = build_instruction(0x9, 0x1, 0x2, 0x3); // AND R1, R2 -> R3

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0x0F00);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_orr() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // ORR: RS1 or RS2 -> RD
        cpu.registers[1] = 0xF000;
        cpu.registers[2] = 0x00F0;

        memory.program_rom[0] = build_instruction(0xA, 0x1, 0x2, 0x3); // ORR R1, R2 -> R3

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0xF0F0);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_xor() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // XOR: RS1 xor RS2 -> RD
        cpu.registers[1] = 0xFF00;
        cpu.registers[2] = 0x0F0F;

        memory.program_rom[0] = build_instruction(0xB, 0x1, 0x2, 0x3); // XOR R1, R2 -> R3

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0xF00F);
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_nor() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // NOR: RS1 nor RS2 -> RD
        cpu.registers[1] = 0x0000;
        cpu.registers[2] = 0x0000;

        memory.program_rom[0] = build_instruction(0xC, 0x1, 0x2, 0x3); // NOR R1, R2 -> R3

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[3], 0xFFFF); // NOR(0, 0) = 1
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_nor_nonzero() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        cpu.registers[1] = 0xAAAA;
        cpu.registers[2] = 0x5555;

        memory.program_rom[0] = build_instruction(0xC, 0x1, 0x2, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        // NOR(0xAAAA, 0x5555) = !(0xAAAA | 0x5555) = !0xFFFF = 0x0000
        assert_eq!(cpu.registers[3], 0x0000);
    }

    #[test]
    fn test_shf_left() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SHF: RS1 shifted by immd4 -> RD
        // immd4 format: DAAA (D=direction, AAA=amount-1)
        // Left shift: D=0, amount=3 (AAA=2)
        cpu.registers[1] = 0x0001;

        // SHF R1, 0x02 -> R2 (left shift by 3)
        // da = 0x02 = 0b00010 = D=0, AAA=2 -> amount=3
        memory.program_rom[0] = build_instruction(0xD, 0x1, 0x2, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x0008); // 1 << 3 = 8
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_shf_left_carry() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // Left shift with carry
        cpu.registers[1] = 0x8000; // Bit 15 set

        // SHF left by 1 (AAA=0)
        memory.program_rom[0] = build_instruction(0xD, 0x1, 0x0, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x0000); // Wraps around
        assert!(cpu.carry); // Bit 15 shifted out
    }

    #[test]
    fn test_shf_right() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // Right shift: D=1, amount=2 (AAA=1)
        cpu.registers[1] = 0x0008;

        // SHF R1, 0x09 -> R2 (right shift by 2)
        // da = 0x09 = 0b01001 = D=1, AAA=1 -> amount=2
        memory.program_rom[0] = build_instruction(0xD, 0x1, 0x9, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x0002); // 8 >> 2 = 2
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_shf_right_carry() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // Right shift with carry
        cpu.registers[1] = 0x0003; // Bits 0 and 1 set

        // SHF right by 1 (D=1, AAA=0)
        memory.program_rom[0] = build_instruction(0xD, 0x1, 0x8, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x0001);
        assert!(cpu.carry); // Bit 0 shifted out
    }

    #[test]
    fn test_shf_max_left() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SHF left by maximum amount (8)
        // D=0, AAA=7 -> amount=8
        cpu.registers[1] = 0x0001;

        memory.program_rom[0] = build_instruction(0xD, 0x1, 0x7, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x0100); // 1 << 8 = 256
    }

    #[test]
    fn test_shf_max_right() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // SHF right by maximum amount (8)
        // D=1, AAA=7 -> amount=8
        cpu.registers[1] = 0xFF00;

        memory.program_rom[0] = build_instruction(0xD, 0x1, 0xF, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.registers[2], 0x00FF); // 0xFF00 >> 8 = 0x00FF
    }

    #[test]
    fn test_brv_negative() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRV: if (RS1 matches NZP) then (RS2 -> PC)
        // Condition bits: 0NZP (bit 2=negative, bit 1=zero, bit 0=positive)
        cpu.registers[1] = 0x8000; // Negative value
        cpu.registers[2] = 0x1234; // Jump target

        // BRV R1, R2, cond=0x4 (negative bit set)
        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x4);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x1234)));
        assert_eq!(cpu.pc, 0x1234);
    }

    #[test]
    fn test_brv_zero() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        cpu.registers[1] = 0x0000; // Zero
        cpu.registers[2] = 0x5678;

        // BRV R1, R2, cond=0x2 (zero bit set)
        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x5678)));
        assert_eq!(cpu.pc, 0x5678);
    }

    #[test]
    fn test_brv_positive() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        cpu.registers[1] = 0x1234; // Positive
        cpu.registers[2] = 0x9ABC;

        // BRV R1, R2, cond=0x1 (positive bit set)
        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x1);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x9ABC)));
        assert_eq!(cpu.pc, 0x9ABC);
    }

    #[test]
    fn test_brv_no_jump() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        cpu.registers[1] = 0x1234; // Positive
        cpu.registers[2] = 0x9ABC;

        // BRV R1, R2, cond=0x4 (negative bit, but value is positive)
        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x4);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.pc, 1); // PC advances normally
    }

    #[test]
    fn test_brv_multiple_conditions() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // Test with multiple condition bits set
        cpu.registers[1] = 0x8000; // Negative
        cpu.registers[2] = 0xDEAD;

        // BRV R1, R2, cond=0x7 (all bits: negative, zero, positive) = unconditional jump
        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x7);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0xDEAD))); // Should jump on negative
    }

    #[test]
    fn test_brv_unconditional_jump() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRV with cond=0x7 (0111) = unconditional jump (jump if Neg, Zero, or Positive)
        cpu.registers[1] = 0x1234; // Any value
        cpu.registers[2] = 0xABCD;

        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x7);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0xABCD)));
    }

    #[test]
    fn test_brv_not_positive() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRV with cond=0x6 (0110) = jump if not positive (negative or zero)
        cpu.registers[1] = 0x8000; // Negative
        cpu.registers[2] = 0x1111;

        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x6);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x1111)));
    }

    #[test]
    fn test_brv_not_zero() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRV with cond=0x5 (0101) = jump if not zero (negative or positive)
        cpu.registers[1] = 0x1234; // Non-zero
        cpu.registers[2] = 0x2222;

        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x5);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x2222)));
    }

    #[test]
    fn test_brv_not_negative() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRV with cond=0x3 (0011) = jump if not negative (zero or positive)
        cpu.registers[1] = 0x1234; // Positive
        cpu.registers[2] = 0x3333;

        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x3333)));
    }

    #[test]
    fn test_brv_nop() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRV with cond=0x0 (0000) = never jump (NOP)
        cpu.registers[1] = 0x1234; // Any value
        cpu.registers[2] = 0x4444;

        memory.program_rom[0] = build_instruction(0xE, 0x1, 0x2, 0x0);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_brf_carry() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRF: if (C or V is set) then (RS2 -> PC)
        // Condition bits: 00VC (bit 1=overflow, bit 0=carry)
        cpu.carry = true;
        cpu.registers[2] = 0x1111;

        // BRF R2, cond=0x1 (carry bit set)
        memory.program_rom[0] = build_instruction(0xF, 0x0, 0x2, 0x1);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x1111)));
        assert_eq!(cpu.pc, 0x1111);
    }

    #[test]
    fn test_brf_overflow() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        cpu.overflow = true;
        cpu.registers[2] = 0x2222;

        // BRF R2, cond=0x2 (overflow bit set)
        memory.program_rom[0] = build_instruction(0xF, 0x0, 0x2, 0x2);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x2222)));
        assert_eq!(cpu.pc, 0x2222);
    }

    #[test]
    fn test_brf_no_flags() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        // BRF with cond=0 means jump if neither carry nor overflow
        cpu.carry = false;
        cpu.overflow = false;
        cpu.registers[2] = 0x3333;

        // BRF R2, cond=0x0 (neither flag set)
        memory.program_rom[0] = build_instruction(0xF, 0x0, 0x2, 0x0);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x3333)));
        assert_eq!(cpu.pc, 0x3333);
    }

    #[test]
    fn test_brf_no_jump() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        cpu.carry = false;
        cpu.overflow = false;
        cpu.registers[2] = 0x4444;

        // BRF R2, cond=0x1 (carry bit, but carry is false)
        memory.program_rom[0] = build_instruction(0xF, 0x0, 0x2, 0x1);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Next));
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn test_brf_both_flags() {
        let mut cpu = Cpu::new();
        let mut memory = create_test_memory();

        cpu.carry = true;
        cpu.overflow = true;
        cpu.registers[2] = 0x5555;

        // BRF R2, cond=0x3 (both carry and overflow bits)
        memory.program_rom[0] = build_instruction(0xF, 0x0, 0x2, 0x3);

        let result = cpu.step(&mut memory).unwrap();
        assert!(matches!(result, InstructionResult::Jump(0x5555))); // Should jump on carry
    }
}
