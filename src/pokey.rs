

struct Pokey {
    audctl: u8,
    audf: [u8; 4],
    audc: [u8; 4],
    pot: [u8; 8],
    allpot: u8,
    stimer: u8,
    kbcode: u8,
    skrest: u8,
    random: u8,
    potgo: u8,
    serout: u8,
    serin: u8,
    irqen: u8,
    irqst: u8,
    skctl: u8,
    skstat: u8,


}