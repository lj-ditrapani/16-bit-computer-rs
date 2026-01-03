use clap::Parser;
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod cpu;
mod error;
mod memory;

use cpu::Cpu;
use error::{CartridgeError, CpuError};
use memory::{Memory, load_cartridge_into_memory};

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
            Ok(cpu::FrameResult::Complete(inst_count)) => {
                if cli.debug {
                    println!("Frame complete: {} instructions", inst_count);
                }
            }
            Ok(cpu::FrameResult::Halted(inst_count)) => {
                println!("CPU halted after {} instructions", inst_count);
                break;
            }
            Err(e) => {
                eprintln!("CPU error: {}", e);
                return Err(e.into());
            }
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
