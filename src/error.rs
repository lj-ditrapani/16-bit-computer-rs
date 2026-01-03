use thiserror::Error;

#[derive(Debug, Error)]
pub enum CpuError {
    #[error("Invalid instruction: {0:04X}")]
    InvalidInstruction(u16),

    #[error("Invalid register index: {0}")]
    InvalidRegister(usize),

    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),
}

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Invalid address: {0:04X}")]
    InvalidAddress(u16),

    #[error("Read-only memory at address: {0:04X}")]
    ReadOnly(u16),

    #[error("Address out of bounds: {0:04X}")]
    OutOfBounds(u16),
}

#[derive(Debug, Error)]
pub enum CartridgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid cartridge file size: {0} bytes (expected 196608 bytes)")]
    InvalidSize(usize),
}
