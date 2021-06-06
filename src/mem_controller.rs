use crate::addressable::Addressable;


pub struct MemController {
    range_addressables: Vec<(Range<u16>, Box<dyn Addressable>)>,
    default_addressable: Box<dyn Addressable>,
}

struct StubAddressable {
}

impl StubAddressable {
    fn new() -> StubAddressable {
        StubAddressable{}
    }
}

impl Addressable for StubAddressable {
    fn get_byte(&self, addr: u16) -> u8 {
        0_u8
    }
    fn set_byte(&mut self, addr: u16, val: u8) {
    }
}

impl MemController {
    pub fn new() -> MemController {
        MemController {
            range_addressables: Vec::new(),
            default_addressable: StubAddressable::new(),
        }
    }

    pub fn register_range(&mut self, range: Range<u16>, addressable: Box<dyn Addressable>) {
        self.range_addressables.add((range, addressable));
    }

    pub fn register_default(&mut self, addressable: Box<dyn Addressable>) {
        self.default_addressable = addressable;
    }

    pub fn get_target(&self, addr: u16) -> Box<Addressable> {
        for (range, addressable) in range_addressables {
            if range.contains(addr) {
                addressable
            }
        }

        default_addressable
    }
}

impl MemController for Addressable {
    fn get_byte(&self, addr: u16) -> u8 {
        for (range, target) in self.range_addressables {
            if range.contains(addr)  {
                return target.get_byte(addr);
            }
        }

        return self.default_addressable.get_byte(addr);
    }
    fn set_byte(&mut self, addr: u16, val: u8) {
        for (range, target) in self.range_addressables {
            if range.contains(addr) {
                target.set_byte(addr, val);
            }
        }

        return self.default_addressable.set_byte(addr, val);
    }
}
