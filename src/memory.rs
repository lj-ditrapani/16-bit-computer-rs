use crate::error::{CartridgeError, MemoryError};
use std::path::Path;

pub struct Memory {
    // Program ROM (64 KW = 128 KB)
    pub program_rom: [u16; 65536],

    // Data ROM (32 KW = 64 KB) - cartridge
    pub data_rom: [u16; 32768],

    // Console RAM (32 KW = 64 KB)
    // $0000-$7FFF: Cartridge Data ROM (read-only mapping)
    // $8000-$EFFF: General RAM (28 KW)
    // $F000-$FFFF: I/O Memory (4 KW)
    pub ram: [u16; 32768],
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            program_rom: [0u16; 65536],
            data_rom: [0u16; 32768],
            ram: [0u16; 32768],
        }
    }

    // Read from program ROM
    pub fn read_program(&self, address: u16) -> Result<u16, MemoryError> {
        Ok(self.program_rom[address as usize])
    }

    // Read from data space
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

    pub fn dump_state(&self) {
        println!("Memory state:");
        println!("  Program ROM: {} words", self.program_rom.len());
        println!("  Data ROM: {} words", self.data_rom.len());
        println!("  RAM: {} words", self.ram.len());
    }
}

pub fn load_cartridge_into_memory(path: &Path) -> Result<Memory, CartridgeError> {
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
