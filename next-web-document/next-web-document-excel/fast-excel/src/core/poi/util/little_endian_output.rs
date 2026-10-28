use std::io;

/// Interface for writing little-endian binary data
pub trait LittleEndianOutput {
    /// Write a byte
    ///
    /// # Arguments
    /// * `v` - Byte value to write
    fn write_byte(&mut self, v: u8) -> io::Result<()>;

    /// Write a short (2 bytes)
    ///
    /// # Arguments
    /// * `v` - Short value to write
    fn write_short(&mut self, v: u16) -> io::Result<()>;

    /// Write an int (4 bytes)
    ///
    /// # Arguments
    /// * `v` - Int value to write
    fn write_int(&mut self, v: u32) -> io::Result<()>;

    /// Write a long (8 bytes)
    ///
    /// # Arguments
    /// * `v` - Long value to write
    fn write_long(&mut self, v: u64) -> io::Result<()>;

    /// Write a double (8 bytes)
    ///
    /// # Arguments
    /// * `v` - Double value to write
    fn write_double(&mut self, v: f64) -> io::Result<()>;

    /// Write a byte array
    ///
    /// # Arguments
    /// * `b` - Byte array to write
    fn write_bytes(&mut self, b: &[u8]) -> io::Result<()>;

    /// Write a portion of a byte array
    ///
    /// # Arguments
    /// * `b` - Byte array to write from
    /// * `offset` - Starting offset in the byte array
    /// * `len` - Number of bytes to write
    fn write_bytes_slice(&mut self, b: &[u8], offset: usize, len: usize) -> io::Result<()>;
}
