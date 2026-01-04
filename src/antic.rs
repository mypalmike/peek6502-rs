use crate::mem::Mem;

/// ANTIC - Alphanumeric Television Interface Controller
/// Handles display list processing, DMA, and video timing for Atari 8-bit computers.
///
/// Memory map: $D400-$D4FF
pub struct Antic {
    // Display list pointer
    dlist_ptr: u16,

    // DMA control
    dma_enabled: bool,

    // Scanline and horizontal position tracking
    scanline: u16,
    horizontal_pos: u8,

    // ANTIC registers
    dmactl: u8,     // $D400 - DMA control
    chactl: u8,     // $D401 - Character mode control
    dlistl: u8,     // $D402 - Display list pointer low
    dlisth: u8,     // $D403 - Display list pointer high
    hscrol: u8,     // $D404 - Horizontal scroll
    vscrol: u8,     // $D405 - Vertical scroll
    pmbase: u8,     // $D407 - Player/missile base address
    chbase: u8,     // $D409 - Character set base address
    wsync: u8,      // $D40A - Wait for horizontal sync
    vcount: u8,     // $D40B - Vertical line counter
    penh: u8,       // $D40C - Light pen horizontal position
    penv: u8,       // $D40D - Light pen vertical position
    nmien: u8,      // $D40E - NMI enable
    nmires: u8,     // $D40F - NMI reset/status
}

impl Antic {
    pub fn new() -> Antic {
        Antic {
            dlist_ptr: 0,
            dma_enabled: false,
            scanline: 0,
            horizontal_pos: 0,
            dmactl: 0,
            chactl: 0,
            dlistl: 0,
            dlisth: 0,
            hscrol: 0,
            vscrol: 0,
            pmbase: 0,
            chbase: 0,
            wsync: 0,
            vcount: 0,
            penh: 0,
            penv: 0,
            nmien: 0,
            nmires: 0,
        }
    }

    /// Execute one machine cycle of ANTIC operation.
    /// Returns true if ANTIC is performing DMA (CPU should be halted).
    pub fn tick(&mut self, _mem: &mut Mem) -> bool {
        self.horizontal_pos += 1;

        // One scanline = 114 color clocks (NTSC)
        if self.horizontal_pos >= 114 {
            self.horizontal_pos = 0;
            self.scanline += 1;
            self.vcount = (self.scanline & 0xff) as u8;

            // Reset at end of frame (262 scanlines for NTSC)
            if self.scanline >= 262 {
                self.scanline = 0;
            }
        }

        // TODO: Implement actual DMA logic
        // For now, just return false (no DMA active)
        false
    }

    /// Read from an ANTIC register
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr & 0x0F {
            0x0B => self.vcount,    // VCOUNT is readable
            0x0C => self.penh,      // Light pen H
            0x0D => self.penv,      // Light pen V
            0x0F => self.nmires,    // NMI status
            _ => 0xFF,              // Other registers are write-only
        }
    }

    /// Write to an ANTIC register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x0F {
            0x00 => {
                self.dmactl = val;
                self.dma_enabled = (val & 0x20) != 0; // Bit 5 enables DMA
            }
            0x01 => self.chactl = val,
            0x02 => {
                self.dlistl = val;
                self.update_dlist_ptr();
            }
            0x03 => {
                self.dlisth = val;
                self.update_dlist_ptr();
            }
            0x04 => self.hscrol = val,
            0x05 => self.vscrol = val,
            0x07 => self.pmbase = val,
            0x09 => self.chbase = val,
            0x0A => self.wsync = val,   // CPU write to WSYNC halts until HSYNC
            0x0E => self.nmien = val,
            0x0F => self.nmires = val,
            _ => {}
        }
    }

    fn update_dlist_ptr(&mut self) {
        self.dlist_ptr = (self.dlistl as u16) | ((self.dlisth as u16) << 8);
    }
}
