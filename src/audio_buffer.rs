/// AudioBuffer collects i16 mono audio samples for one frame.
/// Analogous to Framebuffer for video output.
/// Reusable across all platforms (Atari POKEY, C64 SID, etc.).
pub struct AudioBuffer {
    pub samples: Vec<i16>,
}

impl AudioBuffer {
    pub fn new() -> AudioBuffer {
        AudioBuffer {
            // Pre-allocate for ~2 frames worth of samples (safety margin)
            samples: Vec::with_capacity(1600),
        }
    }

    /// Push one sample into the buffer
    pub fn push(&mut self, sample: i16) {
        self.samples.push(sample);
    }

    /// Clear the buffer at the start of each frame
    pub fn start_frame(&mut self) {
        self.samples.clear();
    }

    /// Get the collected samples as a slice
    pub fn as_slice(&self) -> &[i16] {
        &self.samples
    }

    /// Number of samples collected so far
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}
