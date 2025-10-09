/// Swaps nibbles in a byte. Needed as bytes are usually displayed in big-endian order and we
/// need little endian.
pub fn swap_nibbles(byte: u8) -> u8 {
    ((byte & 0x0f) << 4) | ((byte & 0xf0) >> 4)
}

/// Write byte in lower-hex, little-endian format to the string provided.
pub fn to_lower_hex(buffer: &mut String, byte: &u8) {
    std::fmt::write(buffer, format_args!("{:x}{:x}", byte & 15, byte >> 4 & 15))
        .expect("write must succeed.");
}

/// Write byte in upper-hex, little-endian format to the string provided.
pub fn to_upper_hex(buffer: &mut String, byte: &u8) {
    std::fmt::write(buffer, format_args!("{:X}{:X}", byte & 15, byte >> 4 & 15))
        .expect("write must succeed.");
}

/// Write byte in binary format to the string provided.
pub fn to_binary(buffer: &mut String, byte: &u8) {
    std::fmt::write(buffer, format_args!("{:b}{:b}", byte & 15, byte >> 4 & 15))
        .expect("write must succeed.");
}

pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Reset,
    Bold,
}

impl Color {
    pub fn ansi(&self) -> &'static str {
        match self {
            Color::Black => "\u{1b}[90m",
            Color::Red => "\u{1b}[91m",
            Color::Green => "\u{1b}[92m",
            Color::Yellow => "\u{1b}[93m",
            Color::Blue => "\u{1b}[94m",
            Color::Magenta => "\u{1b}[95m",
            Color::Cyan => "\u{1b}[96m",
            Color::White => "\u{1b}[97m",
            Color::Reset => "\u{1b}[0m",
            Color::Bold => "\u{1b}[1m",
        }
    }
}
