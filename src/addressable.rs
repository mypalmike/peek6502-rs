
pub trait Addressable {
    fn get_byte(&self, addr: u16) -> u8;
    fn set_byte(&mut self, addr: u16, val: u8);

    fn get_word(&self, addr: u16) -> u16 {
        let addr1 = self.get_byte(addr) as u16;
        let addr2 = (self.get_byte(addr + 1) as u16) << 8;
        addr1 | addr2
    }

    fn set_word(&mut self, addr: u16, val: u16) {
        let lo_byte = (val & 0xff) as u8;
        let hi_byte = (val >> 8) as u8;
        self.set_byte(addr, lo_byte);
        self.set_byte(addr + 1, hi_byte);
    }

    fn get_8_bytes(&self, addr: u16) -> [u8; 8] {
        let mut result: [u8; 8] = [0; 8];

        for i in 0..8 {
            result[i] = self.get_byte(addr + (i as u16));
        }

        result
    }
}
