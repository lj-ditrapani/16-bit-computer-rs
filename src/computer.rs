use crate::cpu::{Cpu, FrameResult, InstructionResult};
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

    pub fn execute_frame(&mut self) -> Result<FrameResult, MemoryError> {
        const INSTRUCTIONS_PER_FRAME: u32 = 34_440;
        let mut instructions_executed = 0;

        for _ in 0..INSTRUCTIONS_PER_FRAME {
            let instruction_result = self.cpu.step(&mut self.memory);
            instructions_executed += 1;
            if matches!(instruction_result, Ok(InstructionResult::Halt)) {
                return Ok(FrameResult::Halted(instructions_executed));
            }
        }

        Ok(FrameResult::Complete(instructions_executed))
    }

    pub fn run(&mut self, debug: bool) -> Result<(), MemoryError> {
        let frame_duration = Duration::from_nanos(16_683_333); // 16.6833 ms
        loop {
            let frame_start = Instant::now();

            // Execute frame
            match self.execute_frame() {
                Ok(FrameResult::Complete(inst_count)) => {
                    if debug {
                        println!("Frame complete: {} instructions", inst_count);
                    }
                }
                Ok(FrameResult::Halted(inst_count)) => {
                    println!("CPU halted after {} instructions", inst_count);
                    break;
                }
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

        Ok(())
    }
}
