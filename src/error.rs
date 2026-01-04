use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Read-only memory at address: {0:04X}")]
    ReadOnly(u16),
}

#[derive(Debug, Error)]
pub enum CartridgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid cartridge file size: {0} bytes (expected 196608 bytes)")]
    InvalidSize(usize),
}
