# Architecture Design: LJD 16-bit Computer Emulator

## Overview

This document describes the architecture for implementing the LJD 16-bit computer emulator in Rust. The initial implementation focuses on the CPU and memory subsystems as a command-line application.

## System Architecture

### High-Level Design

The emulator follows a modular design with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│         Command Line Interface          │
│  (CLI argument parsing, main loop)      │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│            Emulator Core                │
│  ┌──────────┐  ┌──────────┐             │
│  │   CPU    │  │  Memory  │             │
│  └──────────┘  └──────────┘             │
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
- **ADI/SBI/SHF**: RS1 is source, RS2 contains 4-bit immediate, RD is destination
- **BRV**: RS1 is register containing value to test (checks sign directly), RS2 is jump target, RD contains condition bits
- **BRF**: RS1 unused (0), RS2 is jump target, RD contains condition bits (checks C/V flags)

### Instruction Set Implementation

Each instruction will be implemented as a method on the `Cpu` struct:

```rust
impl Cpu {
    fn execute_instruction(&mut self, instruction: u16, memory: &mut Memory) -> Result<(), CpuError> {
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
        }
    }
}
```

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
    fn read_program(&self, address: u16) -> Result<u16, MemoryError> {
        // Bounds check and return instruction
    }
    
    // Read from data space
    fn read_data(&self, address: u16) -> Result<u16, MemoryError> {
        match address {
            0x0000..=0x7FFF => Ok(self.data_rom[address as usize]),
            0x8000..=0xFFFF => Ok(self.ram[(address - 0x8000) as usize]),
            _ => Err(MemoryError::InvalidAddress),
        }
    }
    
    // Write to data space (only RAM, ROM is read-only)
    fn write_data(&mut self, address: u16, value: u16) -> Result<(), MemoryError> {
        match address {
            0x0000..=0x7FFF => Err(MemoryError::ReadOnly),
            0x8000..=0xFFFF => {
                self.ram[(address - 0x8000) as usize] = value;
                Ok(())
            },
            _ => Err(MemoryError::InvalidAddress),
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
impl Memory {
    fn new() -> Self {
        Memory {
            program_rom: [0u16; 65536],
            data_rom: [0u16; 32768],
            ram: [0u16; 32768],
        }
    }
}

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
    fn step(&mut self, memory: &mut Memory) -> Result<ExecutionState, CpuError> {
        // Fetch instruction
        let instruction = memory.read_program(self.pc)?;
        
        // Increment PC (unless instruction modifies it)
        let next_pc = self.pc.wrapping_add(1);
        
        // Execute instruction
        let state = self.execute_instruction(instruction, memory)?;
        
        // Update PC (if not modified by instruction)
        if state.pc_modified {
            self.pc = state.new_pc;
        } else {
            self.pc = next_pc;
        }
        
        Ok(state)
    }
    
    fn execute_frame(&mut self, memory: &mut Memory) -> Result<FrameResult, CpuError> {
        const INSTRUCTIONS_PER_FRAME: u32 = 34_440;
        let mut instructions_executed = 0;
        
        for _ in 0..INSTRUCTIONS_PER_FRAME {
            match self.step(memory)? {
                ExecutionState::Running => {
                    instructions_executed += 1;
                },
                ExecutionState::Halted => {
                    return Ok(FrameResult::Halted(instructions_executed));
                },
            }
        }
        
        Ok(FrameResult::Complete(instructions_executed))
    }
}
```

### Execution States

```rust
enum ExecutionState {
    Running,
    Halted,  // END instruction executed
}

enum FrameResult {
    Complete(u32),  // Frame completed, number of instructions executed
    Halted(u32),    // END instruction reached, number of instructions executed
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
    
    /// Number of instructions to execute (0 = unlimited)
    #[arg(short, long, default_value = "0")]
    instructions: u64,
    
    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
    
    /// Dump CPU state after execution
    #[arg(long)]
    dump: bool,
}
```

### Main Execution Loop

```rust
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Load cartridge into memory
    let mut memory = load_cartridge_into_memory(&cli.cartridge)?;
    
    // Initialize CPU
    let mut cpu = Cpu::new();
    
    // Frame-based execution loop
    let frame_duration = Duration::from_nanos(16_683_333); // 16.6833 ms
    loop {
        let frame_start = Instant::now();
        
        // Execute frame
        match cpu.execute_frame(&mut memory) {
            Ok(FrameResult::Complete(inst_count)) => {
                if cli.debug {
                    println!("Frame complete: {} instructions", inst_count);
                }
            },
            Ok(FrameResult::Halted(inst_count)) => {
                println!("CPU halted after {} instructions", inst_count);
                break;
            },
            Err(e) => {
                eprintln!("CPU error: {}", e);
                return Err(e.into());
            },
        }
        
        // TODO: Process IO (video, audio, input)
        
        // Sleep for remainder of frame time
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
    
    if cli.dump {
        cpu.dump_state();
        memory.dump_state();
    }
    
    Ok(())
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
enum CpuError {
    #[error("Invalid instruction: {0:04X}")]
    InvalidInstruction(u16),
    
    #[error("Invalid register index: {0}")]
    InvalidRegister(usize),
    
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),
}

#[derive(Debug, thiserror::Error)]
enum MemoryError {
    #[error("Invalid address: {0:04X}")]
    InvalidAddress(u16),
    
    #[error("Read-only memory at address: {0:04X}")]
    ReadOnly(u16),
    
    #[error("Address out of bounds: {0:04X}")]
    OutOfBounds(u16),
}

#[derive(Debug, thiserror::Error)]
enum CartridgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid cartridge file size: {0} bytes (expected 196608 bytes)")]
    InvalidSize(usize),
}
```

## Module Structure

```
src/
├── main.rs              # CLI entry point
├── cpu/
│   ├── mod.rs           # CPU module
│   ├── registers.rs     # Register and flag management
│   └── instructions.rs  # Instruction implementations
├── memory/
│   ├── mod.rs           # Memory module
│   ├── rom.rs           # ROM handling
│   └── ram.rs           # RAM handling
└── error.rs             # Error types
```

## Implementation Phases

### Phase 1: Core CPU and Memory
- [x] CPU register structure
- [ ] Basic instruction execution
- [ ] Memory read/write
- [ ] Instruction fetch
- [ ] Frame-based execution loop
- [ ] Simple CLI

### Phase 2: Complete Instruction Set
- [ ] All 16 instructions implemented
- [ ] Flag management
- [ ] Branch instructions
- [ ] Shift operations

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

## Future Considerations

- **Performance**: Using fixed-size `u16` arrays for memory access (no heap allocation, better cache locality)
- **Timing**: Frame-based execution (not cycle-perfect) - simpler implementation, adequate for most use cases
- **Debugging**: Add breakpoint support and step-by-step execution
- **Cartridge Format**: Binary format is defined (192 KB fixed size)
- **Save States**: Implement save/load state functionality
- **Disassembler**: Add instruction disassembly for debugging
