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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_cartridges_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_cartridges")
    }

    #[test]
    fn test_load_valid_cartridge() {
        let cartridge_path = test_cartridges_dir().join("valid.bin");
        let memory = load_cartridge_into_memory(&cartridge_path).unwrap();

        // Verify Program ROM loaded correctly
        // First word should be 0x1234 (stored as bytes [0x12, 0x34])
        assert_eq!(memory.program_rom[0], 0x1234);
        assert_eq!(memory.program_rom[1], 0x5678);
        assert_eq!(memory.program_rom[2], 0x9ABC);
        assert_eq!(memory.program_rom[3], 0xDEF0);
        // Rest should be zeros
        assert_eq!(memory.program_rom[4], 0x0000);
        assert_eq!(memory.program_rom[65535], 0x0000);

        // Verify Data ROM loaded correctly
        // First word should be 0x1111 (stored as bytes [0x11, 0x11])
        assert_eq!(memory.data_rom[0], 0x1111);
        assert_eq!(memory.data_rom[1], 0x2222);
        assert_eq!(memory.data_rom[2], 0x3333);
        assert_eq!(memory.data_rom[3], 0x4444);
        // Rest should be zeros
        assert_eq!(memory.data_rom[4], 0x0000);
        assert_eq!(memory.data_rom[32767], 0x0000);

        // Verify RAM is initialized to zero
        assert_eq!(memory.ram[0], 0x0000);
        assert_eq!(memory.ram[32767], 0x0000);
    }

    #[test]
    fn test_load_cartridge_byte_order() {
        let cartridge_path = test_cartridges_dir().join("valid.bin");
        let memory = load_cartridge_into_memory(&cartridge_path).unwrap();

        // Verify big-endian byte order
        // 0x1234 should be stored as [0x12, 0x34] in the file
        // and loaded as 0x1234
        assert_eq!(memory.program_rom[0], 0x1234);

        // Verify a word that spans byte boundaries
        // 0x9ABC = [0x9A, 0xBC]
        assert_eq!(memory.program_rom[2], 0x9ABC);
    }

    #[test]
    fn test_load_cartridge_file_not_found() {
        let cartridge_path = test_cartridges_dir().join("nonexistent.bin");
        let result = load_cartridge_into_memory(&cartridge_path);
        assert!(result.is_err());
        if let Err(CartridgeError::Io(_)) = result {
            // Expected error type
        } else {
            panic!("Expected Io error");
        }
    }

    #[test]
    fn test_load_cartridge_wrong_size_too_small() {
        let cartridge_path = test_cartridges_dir().join("too_small.bin");

        let result = load_cartridge_into_memory(&cartridge_path);
        assert!(result.is_err());
        if let Err(CartridgeError::InvalidSize(size)) = result {
            assert_eq!(size, 192 * 1024 - 1);
        } else {
            panic!("Expected InvalidSize error");
        }
    }

    #[test]
    fn test_load_cartridge_wrong_size_too_large() {
        let cartridge_path = test_cartridges_dir().join("too_large.bin");

        let result = load_cartridge_into_memory(&cartridge_path);
        assert!(result.is_err());
        if let Err(CartridgeError::InvalidSize(size)) = result {
            assert_eq!(size, 192 * 1024 + 1);
        } else {
            panic!("Expected InvalidSize error");
        }
    }

    #[test]
    fn test_load_cartridge_empty_file() {
        let cartridge_path = test_cartridges_dir().join("empty.bin");

        let result = load_cartridge_into_memory(&cartridge_path);
        assert!(result.is_err());
        if let Err(CartridgeError::InvalidSize(size)) = result {
            assert_eq!(size, 0);
        } else {
            panic!("Expected InvalidSize error");
        }
    }

    #[test]
    fn test_load_cartridge_boundaries() {
        let cartridge_path = test_cartridges_dir().join("valid.bin");
        let memory = load_cartridge_into_memory(&cartridge_path).unwrap();

        // Verify Program ROM boundaries
        // Last word of Program ROM (at index 65535)
        assert_eq!(memory.program_rom[65535], 0x0000);

        // Verify Data ROM boundaries
        // Last word of Data ROM (at index 32767)
        assert_eq!(memory.data_rom[32767], 0x0000);
    }

    #[test]
    fn test_load_cartridge_program_rom_size() {
        let cartridge_path = test_cartridges_dir().join("valid.bin");
        let memory = load_cartridge_into_memory(&cartridge_path).unwrap();

        // Verify Program ROM is exactly 64 KW (65,536 words)
        assert_eq!(memory.program_rom.len(), 65536);
    }

    #[test]
    fn test_load_cartridge_data_rom_size() {
        let cartridge_path = test_cartridges_dir().join("valid.bin");
        let memory = load_cartridge_into_memory(&cartridge_path).unwrap();

        // Verify Data ROM is exactly 32 KW (32,768 words)
        assert_eq!(memory.data_rom.len(), 32768);
    }

    #[test]
    fn test_load_cartridge_ram_size() {
        let cartridge_path = test_cartridges_dir().join("valid.bin");
        let memory = load_cartridge_into_memory(&cartridge_path).unwrap();

        // Verify RAM is exactly 32 KW (32,768 words)
        assert_eq!(memory.ram.len(), 32768);
    }
}
