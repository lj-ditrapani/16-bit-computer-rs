use crate::error::{CpuError, MemoryError};
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

#[derive(Debug)]
pub enum FrameResult {
    Complete(u32), // Frame completed, number of instructions executed
    Halted(u32),   // END instruction reached, number of instructions executed
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

    pub fn step(&mut self, memory: &mut Memory) -> Result<InstructionResult, CpuError> {
        // Fetch instruction
        let instruction = memory.read_program(self.pc)?;

        // Execute instruction (may modify PC for branch instructions)
        let instruction_result = self.execute_instruction(instruction, memory)?;

        let pc = match instruction_result {
            InstructionResult::Jump(addr) => addr,
            InstructionResult::Next => self.pc.wrapping_add(1),
            InstructionResult::Halt => self.pc,
        };
        self.pc = pc;
        Ok(instruction_result)
    }

    pub fn execute_frame(&mut self, memory: &mut Memory) -> Result<FrameResult, CpuError> {
        const INSTRUCTIONS_PER_FRAME: u32 = 34_440;
        let mut instructions_executed = 0;

        for _ in 0..INSTRUCTIONS_PER_FRAME {
            let instruction_result = self.step(memory);
            instructions_executed += 1;
            if matches!(instruction_result, Ok(InstructionResult::Halt)) {
                return Ok(FrameResult::Halted(instructions_executed));
            }
        }

        Ok(FrameResult::Complete(instructions_executed))
    }

    fn execute_instruction(
        &mut self,
        instruction: u16,
        memory: &mut Memory,
    ) -> Result<InstructionResult, CpuError> {
        let opcode = (instruction >> 12) & 0xF;
        let rs1 = ((instruction >> 8) & 0xF) as usize;
        let rs2 = ((instruction >> 4) & 0xF) as usize;
        let rd = (instruction & 0xF) as usize;

        match opcode {
            0x0 => self.end(),
            0x1 => self.hby(instruction, rd),
            0x2 => self.lby(instruction, rd),
            0x3 => self.lod(rs1, rd, memory),
            0x4 => self.str(rs1, rs2, memory),
            0x5 => self.add(rs1, rs2, rd),
            0x6 => self.sub(rs1, rs2, rd),
            0x7 => self.adi(rs1, instruction, rd),
            0x8 => self.sbi(rs1, instruction, rd),
            0x9 => self.and(rs1, rs2, rd),
            0xA => self.orr(rs1, rs2, rd),
            0xB => self.xor(rs1, rs2, rd),
            0xC => self.nor(rs1, rs2, rd),
            0xD => self.shf(rs1, instruction, rd),
            0xE => self.brv(rs1, rs2, instruction),
            0xF => self.brf(rs2, instruction),
            _ => Err(CpuError::InvalidInstruction(instruction)),
        }
    }

    // Instruction implementations

    fn end(&self) -> Result<InstructionResult, CpuError> {
        Ok(InstructionResult::Halt)
    }

    fn hby(&mut self, instruction: u16, rd: usize) -> Result<InstructionResult, CpuError> {
        // HBY: immd8 -> RD[15-08]
        // RS1+RS2 form 8-bit immediate
        let immd8 = ((instruction >> 4) & 0xFF) as u8;
        self.registers[rd] = (self.registers[rd] & 0x00FF) | ((immd8 as u16) << 8);
        Ok(InstructionResult::Next)
    }

    fn lby(&mut self, instruction: u16, rd: usize) -> Result<InstructionResult, CpuError> {
        // LBY: immd8 -> RD[07-00]
        // RS1+RS2 form 8-bit immediate
        let immd8 = ((instruction >> 4) & 0xFF) as u8;
        self.registers[rd] = (self.registers[rd] & 0xFF00) | (immd8 as u16);
        Ok(InstructionResult::Next)
    }

    fn lod(
        &mut self,
        rs1: usize,
        rd: usize,
        memory: &Memory,
    ) -> Result<InstructionResult, CpuError> {
        // LOD: ram[RS1] -> RD
        let address = self.registers[rs1];
        let value = memory.read_data(address)?;
        self.registers[rd] = value;
        Ok(InstructionResult::Next)
    }

    fn str(
        &mut self,
        rs1: usize,
        rs2: usize,
        memory: &mut Memory,
    ) -> Result<InstructionResult, CpuError> {
        // STR: RS2 -> ram[RS1]
        let address = self.registers[rs1];
        let value = self.registers[rs2];
        memory.write_data(address, value)?;
        Ok(InstructionResult::Next)
    }

    fn add(&mut self, rs1: usize, rs2: usize, rd: usize) -> Result<InstructionResult, CpuError> {
        // ADD: RS1 + RS2 -> RD
        let a = self.registers[rs1] as u32;
        let b = self.registers[rs2] as u32;
        let result = a + b;

        self.registers[rd] = result as u16;
        self.carry = result > 0xFFFF;
        self.overflow = ((a ^ b) & 0x8000) == 0 && ((a ^ result) & 0x8000) != 0;

        Ok(InstructionResult::Next)
    }

    fn sub(&mut self, rs1: usize, rs2: usize, rd: usize) -> Result<InstructionResult, CpuError> {
        // SUB: RS1 - RS2 -> RD
        let a = self.registers[rs1] as u32;
        let b = self.registers[rs2] as u32;
        let result = a.wrapping_sub(b);

        self.registers[rd] = result as u16;
        self.carry = result > 0xFFFF;
        self.overflow = ((a ^ b) & 0x8000) != 0 && ((a ^ result) & 0x8000) != 0;

        Ok(InstructionResult::Next)
    }

    fn adi(
        &mut self,
        rs1: usize,
        instruction: u16,
        rd: usize,
    ) -> Result<InstructionResult, CpuError> {
        // ADI: RS1 + immd4 -> RD
        let immd4 = ((instruction >> 4) & 0xF) as u32;
        let a = self.registers[rs1] as u32;
        let result = a + immd4;

        self.registers[rd] = result as u16;
        self.carry = result > 0xFFFF;
        self.overflow = ((a ^ immd4) & 0x8000) == 0 && ((a ^ result) & 0x8000) != 0;

        Ok(InstructionResult::Next)
    }

    fn sbi(
        &mut self,
        rs1: usize,
        instruction: u16,
        rd: usize,
    ) -> Result<InstructionResult, CpuError> {
        // SBI: RS1 - immd4 -> RD
        let immd4 = ((instruction >> 4) & 0xF) as u32;
        let a = self.registers[rs1] as u32;
        let result = a.wrapping_sub(immd4);

        self.registers[rd] = result as u16;
        self.carry = result > 0xFFFF;
        self.overflow = ((a ^ immd4) & 0x8000) != 0 && ((a ^ result) & 0x8000) != 0;

        Ok(InstructionResult::Next)
    }

    fn and(&mut self, rs1: usize, rs2: usize, rd: usize) -> Result<InstructionResult, CpuError> {
        // AND: RS1 and RS2 -> RD
        self.registers[rd] = self.registers[rs1] & self.registers[rs2];
        Ok(InstructionResult::Next)
    }

    fn orr(&mut self, rs1: usize, rs2: usize, rd: usize) -> Result<InstructionResult, CpuError> {
        // ORR: RS1 or RS2 -> RD
        self.registers[rd] = self.registers[rs1] | self.registers[rs2];
        Ok(InstructionResult::Next)
    }

    fn xor(&mut self, rs1: usize, rs2: usize, rd: usize) -> Result<InstructionResult, CpuError> {
        // XOR: RS1 xor RS2 -> RD
        self.registers[rd] = self.registers[rs1] ^ self.registers[rs2];
        Ok(InstructionResult::Next)
    }

    fn nor(&mut self, rs1: usize, rs2: usize, rd: usize) -> Result<InstructionResult, CpuError> {
        // NOR: RS1 nor RS2 -> RD
        self.registers[rd] = !(self.registers[rs1] | self.registers[rs2]);
        Ok(InstructionResult::Next)
    }

    fn shf(
        &mut self,
        rs1: usize,
        instruction: u16,
        rd: usize,
    ) -> Result<InstructionResult, CpuError> {
        // SHF: RS1 shifted by immd4 -> RD
        // immd4 format: DAAA
        // D is direction: 0 left, 1 right
        // AAA is (amount - 1), so 0-7 -> 1-8
        let immd4 = (instruction >> 4) & 0xF;
        let direction = (immd4 >> 3) & 1;
        let amount = ((immd4 & 0x7) + 1) as u32;

        let value = self.registers[rs1];
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

        self.registers[rd] = result;
        Ok(InstructionResult::Next)
    }

    fn brv(
        &mut self,
        rs1: usize,
        rs2: usize,
        instruction: u16,
    ) -> Result<InstructionResult, CpuError> {
        // BRV: if (RS1 matches NZP) then (RS2 -> PC)
        // RD contains condition bits: 0NZP
        let value = self.registers[rs1];
        let cond = instruction & 0xF;

        // Check sign of value
        let is_negative = (value & 0x8000) != 0;
        let is_zero = value == 0;
        let is_positive = !is_negative && !is_zero;

        // Condition bits: 0NZP (bit 3 unused, bit 2=negative, bit 1=zero, bit 0=positive)
        let should_jump = match cond {
            0x0 => false,        // 0000: never jump (NOP)
            0x1 => is_positive,  // 0001: jump if positive
            0x2 => is_zero,      // 0010: jump if zero
            0x3 => !is_negative, // 0011: jump if not negative
            0x4 => is_negative,  // 0100: jump if negative
            0x5 => !is_zero,     // 0101: jump if not zero
            0x6 => !is_positive, // 0110: jump if not positive
            0x7 => true,         // 0111: unconditional jump
            _ => false,
        };

        if should_jump {
            Ok(InstructionResult::Jump(self.registers[rs2]))
        } else {
            Ok(InstructionResult::Next)
        }
    }

    fn brf(&mut self, rs2: usize, instruction: u16) -> Result<InstructionResult, CpuError> {
        // BRF: if (C or V is set) then (RS2 -> PC)
        // RD contains condition bits: 00VC
        let cond = instruction & 0xF;

        // Condition bits: 00VC (bits 3-2 unused, bit 1=overflow, bit 0=carry)
        let should_jump = match cond {
            0x0 => !self.carry && !self.overflow, // 0000: jump if carry and overflow NOT set
            0x1 => self.carry,                    // 0001: jump if carry set
            0x2 => self.overflow,                 // 0010: jump if overflow set
            0x3 => self.carry || self.overflow,   // 0011: jump if overflow or carry set
            _ => false,
        };

        if should_jump {
            Ok(InstructionResult::Jump(self.registers[rs2]))
        } else {
            Ok(InstructionResult::Next)
        }
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
