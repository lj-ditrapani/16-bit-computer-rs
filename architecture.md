# Architecture Design: LJD 16-bit Computer Emulator

## Overview

This document describes the architecture for implementing the LJD 16-bit computer emulator in Rust. The initial implementation focuses on the CPU and memory subsystems as a command-line application.

## System Architecture

### High-Level Design

The emulator follows a modular design with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│         Command Line Interface          │
│  (CLI argument parsing)                 │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│            Computer                     │
│  ┌──────────┐  ┌──────────┐             │
│  │   CPU    │  │  Memory  │             │
│  └──────────┘  └──────────┘             │
│  (main loop in Computer::run())         │
└─────────────────────────────────────────┘
```

## CPU Architecture

### Registers

The CPU has 16 general-purpose 16-bit registers plus a program counter:

```rust
struct Cpu {
    // General purpose registers R0-R15
    registers: [u16; 16],
    
    // Program counter (16-bit)
    pc: u16,
    
    // Status flags
    carry: bool,      // C flag
    overflow: bool,   // V flag
}
```

### Instruction Format

All instructions are 16 bits:

```
┌─────┬─────┬─────┬─────┐
│ Op  │ RS1 │ RS2 │ RD  │
│(4b) │(4b) │(4b) │(4b) │
└─────┴─────┴─────┴─────┘
```

- **Op**: 4-bit opcode (0x0-0xF)
- **RS1**: Source register 1 (2nd nibble)
- **RS2**: Source register 2 (3rd nibble)  
- **RD**: Destination register (4th nibble)

Some instructions use immediate values in RS1/RS2 positions:
- **HBY/LBY**: RS1+RS2 form 8-bit immediate, RD is destination
- **ADI/SBI**: RS1 is source register, RS2 contains 4-bit immediate, RD is destination
- **SHF**: RS1 is source register, RS2 contains direction+amount (DAAA format), RD is destination
- **BRV**: RS1 is register containing value to test (checks sign directly), RS2 is jump target register, RD contains condition bits (masked to 0x7)
- **BRF**: RS1 unused (0), RS2 is jump target register, RD contains condition bits (masked to 0x7, checks C/V flags)

### Instruction Result

Instructions return an `InstructionResult` enum that indicates how the program counter should be updated:

```rust
#[derive(Debug)]
pub enum InstructionResult {
    Jump(u16), // Set PC to address
    Next,      // Increment PC
    Halt,      // END instruction executed
}
```

### Register Access Helpers

The CPU provides helper methods for register access:

```rust
impl Cpu {
    fn register(&self, index: u8) -> u16 {
        self.registers[index as usize]
    }

    fn set_register(&mut self, index: u8, value: u16) {
        self.registers[index as usize] = value
    }
}
```

### Instruction Set Implementation

Each instruction is implemented as a method on the `Cpu` struct. Register indices are `u8` (0-15):

```rust
impl Cpu {
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
            _ => Ok(self.brf(rs2, rd)), // _ can only be 0xF
        }
    }
}
```

**Note**: Most instructions return `InstructionResult` directly (wrapped in `Ok()`), but `str` returns `Result<InstructionResult, MemoryError>` because it can fail when writing to read-only memory.

### Flag Management

Flags are updated after arithmetic/logical operations:

- **Carry (C)**: Set when addition overflows or subtraction underflows
- **Overflow (V)**: Set when signed arithmetic overflows

**Note**: The CPU does not maintain N/Z/P flags. The BRV instruction checks the sign of the register value directly:
- Negative: MSB is 1 (value >= 0x8000)
- Zero: value == 0
- Positive: MSB is 0 and value != 0 (value < 0x8000 and value != 0)

## Memory Architecture

### Harvard Architecture

The system uses separate address spaces for program and data:

```rust
struct Memory {
    // Program ROM (64 KW = 128 KB)
    program_rom: [u16; 65536],  // 64 KW
    
    // Data ROM (32 KW = 64 KB) - cartridge
    data_rom: [u16; 32768],  // 32 KW
    
    // Console RAM (32 KW = 64 KB)
    // $0000-$7FFF: Cartridge Data ROM (read-only mapping)
    // $8000-$EFFF: General RAM (28 KW)
    // $F000-$FFFF: I/O Memory (4 KW)
    ram: [u16; 32768],  // 32 KW
}
```

### Memory Map

```
Program Address Space (16-bit):
┌─────────────────────────────┐
│ $0000 - $FFFF               │ 64 KW Program ROM
└─────────────────────────────┘

Data Address Space (16-bit):
┌─────────────────────────────┐
│ $0000 - $7FFF               │ 32 KW Cartridge Data ROM
├─────────────────────────────┤
│ $8000 - $EFFF               │ 28 KW General RAM
├─────────────────────────────┤
│ $F000 - $F7FF               │ 2 KW Background Tiles
├─────────────────────────────┤
│ $F800 - $FBFF               │ 1 KW Foreground Tiles
├─────────────────────────────┤
│ $FC00 - $FFFF               │ 1 KW Other I/O Memory
└─────────────────────────────┘
```

### Memory Access Methods

```rust
impl Memory {
    // Read from program ROM
    pub fn read_program(&self, address: u16) -> Result<u16, MemoryError> {
        Ok(self.program_rom[address as usize])
    }

    // Read from data space (never errors - all u16 addresses are valid)
    pub fn read_data(&self, address: u16) -> u16 {
        match address {
            0x0000..=0x7FFF => self.data_rom[address as usize],
            0x8000..=0xFFFF => self.ram[(address - 0x8000) as usize],
        }
    }

    // Write to data space (only RAM, ROM is read-only)
    pub fn write_data(&mut self, address: u16, value: u16) -> Result<(), MemoryError> {
        match address {
            0x0000..=0x7FFF => Err(MemoryError::ReadOnly(address)),
            0x8000..=0xFFFF => {
                self.ram[(address - 0x8000) as usize] = value;
                Ok(())
            }
        }
    }
}
```

### Word-Addressable Memory

All memory is word-addressable (16-bit words). Each address represents one 16-bit word, not a byte.

## Cartridge File Format

The cartridge is a single binary file of fixed size:

```
┌─────────────────────────────────────┐
│ Program ROM                         │  128 KB (64 KW × 2 bytes)
│ $0000 - $FFFF                       │
├─────────────────────────────────────┤
│ Data ROM                            │   64 KB (32 KW × 2 bytes)
│ $0000 - $7FFF                       │
└─────────────────────────────────────┘
Total: 192 KB
```

### File Layout

- **Bytes 0 - 131,071** (0x00000 - 0x1FFFF): Program ROM
  - 64 KW = 65,536 words
  - Each word is 2 bytes: MSB (first byte) + LSB (second byte)
  - Maps to program address space $0000-$FFFF

- **Bytes 131,072 - 196,607** (0x20000 - 0x2FFFF): Data ROM
  - 32 KW = 32,768 words
  - Each word is 2 bytes: MSB (first byte) + LSB (second byte)
  - Maps to data address space $0000-$7FFF

### Loading Implementation

```rust
fn load_cartridge_into_memory(path: &Path) -> Result<Memory, CartridgeError> {
    let data = std::fs::read(path)?;
    
    // Validate file size
    const EXPECTED_SIZE: usize = 192 * 1024; // 192 KB
    if data.len() != EXPECTED_SIZE {
        return Err(CartridgeError::InvalidSize(data.len()));
    }
    
    // Load Program ROM (first 128 KB = 64 KW)
    // Each word is 2 bytes: MSB (first byte) + LSB (second byte)
    let mut program_rom = [0u16; 65536];
    for (i, chunk) in data[0..131072].chunks_exact(2).enumerate() {
        program_rom[i] = u16::from_be_bytes([chunk[0], chunk[1]]);
    }
    
    // Load Data ROM (next 64 KB = 32 KW)
    // Each word is 2 bytes: MSB (first byte) + LSB (second byte)
    let mut data_rom = [0u16; 32768];
    for (i, chunk) in data[131072..196608].chunks_exact(2).enumerate() {
        data_rom[i] = u16::from_be_bytes([chunk[0], chunk[1]]);
    }
    
    // Initialize RAM to zero
    let ram = [0u16; 32768];
    
    Ok(Memory {
        program_rom,
        data_rom,
        ram,
    })
}
```

## Execution Model

### Frame-Based Execution

Instead of cycle-perfect emulation, we use a simplified frame-based approach:

1. **Execute instructions**: Run up to 34,440 instructions per frame (or until END instruction)
2. **Process IO**: Handle video rendering, audio, input, etc.
3. **Sleep**: Sleep for the remainder of the frame time (16.6833 ms total)

**Tradeoff**: This approach simplifies implementation but limits correctness. We can only correctly emulate programs that do not write to IO RAM during the VDP rendering period. Programs that attempt to modify video memory or other IO registers during rendering may not behave correctly.

### CPU Execution

```rust
impl Cpu {
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
}
```

## Computer Architecture

The `Computer` struct encapsulates both the CPU and memory, and contains the main execution loop:

```rust
pub struct Computer {
    pub cpu: Cpu,
    pub memory: Memory,
}

impl Computer {
    pub fn new(memory: Memory) -> Self {
        Computer {
            cpu: Cpu::new(),
            memory,
        }
    }

    pub fn execute_frame(&mut self) -> Result<(), MemoryError> {
        const INSTRUCTIONS_PER_FRAME: u32 = 34_440;

        for _ in 0..INSTRUCTIONS_PER_FRAME {
            let instruction_result = self.cpu.step(&mut self.memory);
            if matches!(instruction_result, Ok(InstructionResult::Halt)) {
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), MemoryError> {
        let frame_duration = Duration::from_nanos(16_683_333); // 16.6833 ms
        loop {
            let frame_start = Instant::now();

            // Execute frame
            match self.execute_frame() {
                Ok(()) => {}
                Err(e) => {
                    return Err(e);
                }
            }

            // TODO: Process IO (video, audio, input)

            // Sleep for remainder of frame time
            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
        }
    }
}
```

## Command Line Interface

### CLI Structure

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "ljd-16-bit-computer-rs")]
#[command(about = "LJD 16-bit Computer Emulator")]
struct Cli {
    /// Path to cartridge ROM file
    #[arg(short, long)]
    cartridge: PathBuf,
}
```

### Main Entry Point

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load cartridge into memory and create computer
    let memory = load_cartridge_into_memory(&cli.cartridge)?;
    let mut computer = Computer::new(memory);

    // Run the computer
    if let Err(e) = computer.run() {
        eprintln!("CPU error: {}", e);
        computer.cpu.dump_state();
        return Err(e.into());
    }

    Ok(())
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Read-only memory at address: {0:04X}")]
    ReadOnly(u16),
}

#[derive(Debug, thiserror::Error)]
pub enum CartridgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid cartridge file size: {0} bytes (expected 196608 bytes)")]
    InvalidSize(usize),
}
```

**Note**: `CpuError` was removed - CPU operations that can error (like `str`) return `MemoryError` directly. Most instructions cannot error and return `InstructionResult` directly.

## Module Structure

```
src/
├── main.rs              # CLI entry point
├── computer.rs          # Computer struct (CPU + Memory + main loop)
├── cpu.rs               # CPU implementation
├── memory.rs            # Memory implementation and cartridge loading
└── error.rs             # Error types
```

## Implementation Phases

### Phase 1: Core CPU and Memory
- [x] CPU register structure
- [x] Basic instruction execution
- [x] Memory read/write
- [x] Instruction fetch
- [x] Frame-based execution loop
- [x] Simple CLI
- [x] Computer struct

### Phase 2: Complete Instruction Set
- [x] All 16 instructions implemented
- [x] Flag management
- [x] Branch instructions
- [x] Shift operations

### Phase 3: Testing and Validation
- [ ] Unit tests for each instruction
- [ ] Integration tests
- [ ] Test with example assembly programs

### Phase 4: Future Enhancements
- [ ] Video system (VDP)
- [ ] Audio system (APU)
- [ ] Gamepad input
- [ ] Timing/cycle accuracy
- [ ] Debugger interface

## Design Decisions

### Why Separate Program/Data Memory?

The Harvard architecture is maintained to match the hardware specification. This allows:
- Clear separation of concerns
- Easier implementation of read-only program memory
- Future optimization opportunities

### Why Word-Addressable?

The specification explicitly states all memory is word-addressable. This simplifies the implementation and matches the hardware design.

### Error Handling Strategy

Using `thiserror` for structured error types that can be easily converted and propagated. All errors are recoverable where possible, with clear error messages for debugging.

## Dependencies

```toml
[dependencies]
clap = { version = "4.0", features = ["derive"] }
thiserror = "1.0"
```

