use crate::cpu::{Cpu, InstructionResult};
use crate::error::MemoryError;
use crate::memory::Memory;
use std::time::{Duration, Instant};

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
            let instruction_result = self.cpu.step(&mut self.memory)?;
            if matches!(instruction_result, InstructionResult::Halt) {
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
