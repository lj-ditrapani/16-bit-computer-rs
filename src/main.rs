use clap::Parser;
use std::path::PathBuf;

mod computer;
mod cpu;
mod error;
mod memory;

use computer::Computer;
use memory::load_cartridge_into_memory;

#[derive(Parser)]
#[command(name = "ljd-16-bit-computer-rs")]
#[command(about = "LJD 16-bit Computer Emulator")]
struct Cli {
    /// Path to cartridge ROM file
    #[arg(short, long)]
    cartridge: PathBuf,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool,

    /// Dump CPU state after execution
    #[arg(long)]
    dump: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load cartridge into memory and create computer
    let memory = load_cartridge_into_memory(&cli.cartridge)?;
    let mut computer = Computer::new(memory);

    // Run the computer
    if let Err(e) = computer.run(cli.debug) {
        eprintln!("CPU error: {}", e);
        computer.cpu.dump_state();
        return Err(e.into());
    }

    if cli.dump {
        computer.cpu.dump_state();
    }

    Ok(())
}
