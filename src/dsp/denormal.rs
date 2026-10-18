//! Add denormal flushing for both tiny results and tiny inputs around audio processing.

pub struct DenormalGuard {
    #[cfg(target_arch = "x86_64")]
    previous_mxcsr: u32,
}

impl DenormalGuard {
    pub fn new() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            let mut previous_mxcsr = 0_u32;
            // SAFETY: MXCSR is thread-local processor state, and the stack slot is writable.
            unsafe {
                std::arch::asm!("stmxcsr [{}]", in(reg) &mut previous_mxcsr);
                std::arch::asm!(
                    "ldmxcsr [{}]",
                    in(reg) &(previous_mxcsr | (1 << 15) | (1 << 6))
                );
            }
            Self { previous_mxcsr }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Self {}
        }
    }
}

impl Drop for DenormalGuard {
    fn drop(&mut self) {
        #[cfg(target_arch = "x86_64")]
        {
            // SAFETY: Restore the exact MXCSR value saved by `new()` on this same thread.
            unsafe {
                std::arch::asm!("ldmxcsr [{}]", in(reg) &self.previous_mxcsr);
            }
        }
    }
}

impl Default for DenormalGuard {
    fn default() -> Self {
        Self::new()
    }
}
