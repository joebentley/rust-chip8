
/// Gets the nth hex digit starting from the least significant digit
///
/// TODO: make this into a macro?
///
/// # Examples
///
/// ```
/// assert_eq!(rust_chip8::get_nth_hex_digit(0x1234, 0), 0x4);
/// assert_eq!(rust_chip8::get_nth_hex_digit(0x1234, 1), 0x3);
/// assert_eq!(rust_chip8::get_nth_hex_digit(0x1234, 2), 0x2);
/// assert_eq!(rust_chip8::get_nth_hex_digit(0x1234, 3), 0x1);
/// ```
pub fn get_nth_hex_digit(hex: u32, n: u32) -> u8 {
    (hex - ((hex >> (n + 1) * 4) << (n + 1) * 4) >> n * 4) as u8
}
