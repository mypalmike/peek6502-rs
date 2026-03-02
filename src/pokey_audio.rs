/// POKEY audio synthesis engine.
///
/// Implements polynomial counter-based sound generation matching real hardware.
///
/// Data flow:
///   CPU writes $D200-$D208 -> PokeyAudio::update_register() (recalcs divider reload values)
///   Per scanline -> PokeyAudio::generate(114, &mut AudioBuffer) (emits ~2.8 samples at 44.1kHz)

use crate::audio_buffer::AudioBuffer;

// ============================================================================
// AUDCTL register bits
// ============================================================================
const AUDCTL_POLY9:      u8 = 0x80; // Use POLY9 instead of POLY17
const AUDCTL_CH1_179:    u8 = 0x40; // Clock channel 1 at 1.79MHz
const AUDCTL_CH3_179:    u8 = 0x20; // Clock channel 3 at 1.79MHz
const AUDCTL_CH1_CH2:    u8 = 0x10; // Link channels 1+2 (16-bit)
const AUDCTL_CH3_CH4:    u8 = 0x08; // Link channels 3+4 (16-bit)
const AUDCTL_CH1_FILTER: u8 = 0x04; // High-pass filter: ch1 clocked by ch3
const AUDCTL_CH2_FILTER: u8 = 0x02; // High-pass filter: ch2 clocked by ch4
const AUDCTL_CLOCK_15:   u8 = 0x01; // Base clock: 15kHz (vs default 64kHz)

// ============================================================================
// AUDC register bits
// ============================================================================
const AUDC_NOTPOLY5: u8 = 0x80; // Skip POLY5 gate (pure clock or direct)
const AUDC_POLY4:    u8 = 0x40; // Use POLY4 (else POLY9/POLY17)
const AUDC_PURETONE: u8 = 0x20; // Pure square wave (no polynomial)
const AUDC_VOL_ONLY: u8 = 0x10; // Volume-only output (DC level / PCM)
const AUDC_VOL_MASK: u8 = 0x0F; // Volume bits

// ============================================================================
// Base clock divisors from 1.79MHz
// ============================================================================
// 1.79MHz / 28  = ~63.9 kHz (64kHz mode, default)
// 1.79MHz / 114 = ~15.7 kHz (15kHz mode)
const DIV_64: u32 = 28;
const DIV_15: u32 = 114;

// ============================================================================
// Polynomial counter tables
// ============================================================================

// POLY4: 15-entry 4-bit LFSR sequence (Atari hardware verified)
const POLY4: [u8; 15] = [1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0, 1, 0];

// POLY5: 31-entry 5-bit LFSR sequence (Atari hardware verified)
const POLY5: [u8; 31] = [
    1, 1, 1, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 0,
    0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0,
];

// ============================================================================
// PokeyAudio struct
// ============================================================================

pub struct PokeyAudio {
    // Register state (mirrors Pokey register file)
    audf: [u8; 4],
    audc: [u8; 4],
    audctl: u8,

    // Per-channel divider state
    div_n_cnt: [u32; 4], // Countdown counter (ticks until next toggle)
    div_n_max: [u32; 4], // Reload value (computed from AUDF/AUDCTL)

    // Per-channel output state
    outvol: [bool; 4], // Current output level: true = high, false = low

    // Polynomial counter positions
    p4:  usize, // Position in POLY4 table  (0..15)
    p5:  usize, // Position in POLY5 table  (0..31)
    p9:  usize, // Position in POLY9 table  (0..511)
    p17: usize, // Position in POLY17 table (0..131071)

    // Pre-computed poly9 and poly17 bit sequences (0 or 1 per entry)
    poly9:  Vec<u8>, // 511 entries
    poly17: Vec<u8>, // 131071 entries

    // Sample rate conversion state
    // Accumulate sample_rate each CPU tick; emit sample when >= cpu_freq
    samp_cnt:    u32,
    sample_rate: u32, // output sample rate (e.g. 44100)
    cpu_freq:    u32, // CPU clock frequency (e.g. 1789790)
}

impl PokeyAudio {
    pub fn new(cpu_freq: u32, sample_rate: u32) -> PokeyAudio {
        let poly9  = build_poly9();
        let poly17 = build_poly17();

        let mut audio = PokeyAudio {
            audf:    [0; 4],
            audc:    [0; 4],
            audctl:  0,
            div_n_cnt: [1; 4],
            div_n_max: [0; 4],
            outvol:    [false; 4],
            p4:  0,
            p5:  0,
            p9:  0,
            p17: 0,
            poly9,
            poly17,
            samp_cnt:    0,
            sample_rate,
            cpu_freq,
        };

        // Initialize dividers with AUDF=0, AUDCTL=0 (64kHz base)
        audio.recalc_dividers();
        audio
    }

    /// Handle a POKEY register write (addresses relative to $D200 base, so 0x00..0x08)
    pub fn update_register(&mut self, addr: u8, val: u8) {
        match addr {
            0x00 => { self.audf[0] = val; self.recalc_ch(0); self.recalc_ch(1); }
            0x01 => { self.audc[0] = val; self.apply_vol_only(0); }
            0x02 => { self.audf[1] = val; self.recalc_ch(1); }
            0x03 => { self.audc[1] = val; self.apply_vol_only(1); }
            0x04 => { self.audf[2] = val; self.recalc_ch(2); self.recalc_ch(3); }
            0x05 => { self.audc[2] = val; self.apply_vol_only(2); }
            0x06 => { self.audf[3] = val; self.recalc_ch(3); }
            0x07 => { self.audc[3] = val; self.apply_vol_only(3); }
            0x08 => { self.audctl = val; self.recalc_dividers(); }
            _ => {}
        }
    }

    /// Generate audio samples for `num_ticks` CPU cycles, pushing to `buffer`.
    /// Called once per scanline with num_ticks=114.
    /// At 44100 Hz output and 59.92 FPS: ~2.81 samples per scanline (736 per frame).
    pub fn generate(&mut self, num_ticks: u32, buffer: &mut AudioBuffer) {
        for _ in 0..num_ticks {
            // Advance polynomial counters every CPU tick
            self.p4  = (self.p4  + 1) % 15;
            self.p5  = (self.p5  + 1) % 31;
            self.p9  = (self.p9  + 1) % 511;
            self.p17 = (self.p17 + 1) % 131071;

            // Advance each channel's divider and handle expiry
            for ch in 0..4 {
                if self.div_n_cnt[ch] > 1 {
                    self.div_n_cnt[ch] -= 1;
                } else if self.div_n_cnt[ch] == 1 {
                    // Counter expired: reload and update output
                    self.div_n_cnt[ch] = self.div_n_max[ch].max(1);
                    self.tick_channel(ch);
                }
                // div_n_cnt == 0 means channel is disabled (VOL_ONLY mode)
            }

            // Sample rate conversion: accumulate sample_rate ticks; emit when >= cpu_freq
            self.samp_cnt += self.sample_rate;
            if self.samp_cnt >= self.cpu_freq {
                self.samp_cnt -= self.cpu_freq;
                buffer.push(self.mix_sample());
            }
        }
    }

    // ========================================================================
    // Private: channel output update
    // ========================================================================

    /// Called when a channel's divider expires. Updates outvol based on AUDC and poly state.
    fn tick_channel(&mut self, ch: usize) {
        let audc = self.audc[ch];

        // High-pass filter: ch3 ticking clears ch1's output (if CH1_FILTER set)
        if ch == 2 && (self.audctl & AUDCTL_CH1_FILTER != 0) {
            self.outvol[0] = false;
        }
        // ch4 ticking clears ch2's output (if CH2_FILTER set)
        if ch == 3 && (self.audctl & AUDCTL_CH2_FILTER != 0) {
            self.outvol[1] = false;
        }

        // VOL_ONLY: output stays high (level determined by volume bits)
        if audc & AUDC_VOL_ONLY != 0 {
            self.outvol[ch] = true;
            return;
        }

        // POLY5 gate: if NOTPOLY5 bit is clear, POLY5 must be 1 for the clock to pass
        let poly5_gate = (audc & AUDC_NOTPOLY5 != 0) || (POLY5[self.p5] != 0);
        if !poly5_gate {
            return; // POLY5 blocking: no toggle this tick
        }

        // Determine whether output should toggle
        let current = self.outvol[ch];
        let toggle = if audc & AUDC_PURETONE != 0 {
            // Pure tone: toggle unconditionally (creates square wave)
            true
        } else if audc & AUDC_POLY4 != 0 {
            // POLY4: output follows poly4 bit; toggle if different from current
            (POLY4[self.p4] != 0) != current
        } else if self.audctl & AUDCTL_POLY9 != 0 {
            // POLY9: 9-bit LFSR noise
            (self.poly9[self.p9] != 0) != current
        } else {
            // POLY17: 17-bit LFSR noise (default)
            (self.poly17[self.p17] != 0) != current
        };

        if toggle {
            self.outvol[ch] = !current;
        }
    }

    /// Mix all 4 channels into a single i16 sample.
    /// Each channel contributes its 4-bit volume when outvol is true.
    fn mix_sample(&self) -> i16 {
        let mut sum: i32 = 0;
        for ch in 0..4 {
            let vol = (self.audc[ch] & AUDC_VOL_MASK) as i32;
            if self.outvol[ch] && vol > 0 {
                sum += vol;
            }
        }
        // Scale 0..60 to roughly 0..30000 (fits in i16, allows headroom)
        // Max sum = 4 channels * 15 volume = 60
        (sum * 500).min(i16::MAX as i32) as i16
    }

    // ========================================================================
    // Private: divider reload value calculation
    // ========================================================================

    /// Recalculate all channel dividers (called on AUDCTL change).
    fn recalc_dividers(&mut self) {
        for ch in 0..4 {
            self.recalc_ch(ch);
        }
    }

    /// Recalculate divider for channel `ch` based on current AUDF and AUDCTL.
    fn recalc_ch(&mut self, ch: usize) {
        let base_mult = if self.audctl & AUDCTL_CLOCK_15 != 0 { DIV_15 } else { DIV_64 };
        let new_max = match ch {
            0 => {
                // Channel 1
                if self.audctl & AUDCTL_CH1_179 != 0 {
                    self.audf[0] as u32 + 4
                } else {
                    (self.audf[0] as u32 + 1) * base_mult
                }
            }
            1 => {
                // Channel 2 (possibly 16-bit linked with channel 1)
                if self.audctl & AUDCTL_CH1_CH2 != 0 {
                    if self.audctl & AUDCTL_CH1_179 != 0 {
                        self.audf[1] as u32 * 256 + self.audf[0] as u32 + 7
                    } else {
                        (self.audf[1] as u32 * 256 + self.audf[0] as u32 + 1) * base_mult
                    }
                } else {
                    (self.audf[1] as u32 + 1) * base_mult
                }
            }
            2 => {
                // Channel 3
                if self.audctl & AUDCTL_CH3_179 != 0 {
                    self.audf[2] as u32 + 4
                } else {
                    (self.audf[2] as u32 + 1) * base_mult
                }
            }
            3 => {
                // Channel 4 (possibly 16-bit linked with channel 3)
                if self.audctl & AUDCTL_CH3_CH4 != 0 {
                    if self.audctl & AUDCTL_CH3_179 != 0 {
                        self.audf[3] as u32 * 256 + self.audf[2] as u32 + 7
                    } else {
                        (self.audf[3] as u32 * 256 + self.audf[2] as u32 + 1) * base_mult
                    }
                } else {
                    (self.audf[3] as u32 + 1) * base_mult
                }
            }
            _ => unreachable!(),
        };

        self.div_n_max[ch] = new_max.max(1);

        // If counter exceeds new max, clamp it (prevents stall on frequency decrease)
        if self.div_n_cnt[ch] > new_max {
            self.div_n_cnt[ch] = new_max;
        }

        // Re-apply VOL_ONLY if needed
        self.apply_vol_only(ch);
    }

    /// For VOL_ONLY channels, set output high immediately and freeze the divider.
    fn apply_vol_only(&mut self, ch: usize) {
        if self.audc[ch] & AUDC_VOL_ONLY != 0 {
            self.outvol[ch] = true;
            // Freeze divider to avoid unnecessary ticking
            self.div_n_cnt[ch] = 0; // 0 = permanently disabled
        }
    }
}

// ============================================================================
// Polynomial counter table generation
// ============================================================================

/// Build POLY9 bit table: 511-entry 9-bit LFSR sequence.
/// Feedback: c = ((poly9 >> 3) ^ (poly9 >> 8)) & 1
fn build_poly9() -> Vec<u8> {
    let mut table = Vec::with_capacity(511);
    let mut poly9: u32 = 1;
    for _ in 0..511 {
        table.push((poly9 & 1) as u8);
        let c = ((poly9 >> 3) ^ (poly9 >> 8)) & 1;
        poly9 = ((poly9 << 1) & 511) + c;
    }
    table
}

/// Build POLY17 bit table: 131071-entry 17-bit LFSR sequence.
/// Feedback: c = ((poly17 >> 11) ^ (poly17 >> 16)) & 1
fn build_poly17() -> Vec<u8> {
    let mut table = Vec::with_capacity(131071);
    let mut poly17: u32 = 1;
    for _ in 0..131071 {
        table.push((poly17 & 1) as u8);
        let c = ((poly17 >> 11) ^ (poly17 >> 16)) & 1;
        poly17 = ((poly17 << 1) & 131071) + c;
    }
    table
}
