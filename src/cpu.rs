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
