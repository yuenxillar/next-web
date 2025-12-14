/// Interface for reading little-endian binary data
pub trait LittleEndianInput {
    /// Get the number of bytes available to read
    ///
    /// # Returns
    /// * Number of bytes available
    fn available(&self) -> usize;

    /// Read a single byte
    ///
    /// # Returns
    /// * Byte value
    fn read_byte(&mut self) -> i8;

    /// Read a single unsigned byte
    ///
    /// # Returns
    /// * Byte value
    fn read_u_byte(&mut self) -> u8;

    /// Read a short (2 bytes)
    ///
    /// # Returns
    /// * Short value
    fn read_short(&mut self) -> i16;

    /// Read an unsigned short (2 bytes)
    ///
    /// # Returns
    /// * Unsigned short value as usize
    fn read_u_short(&mut self) -> u16;

    /// Read an int (4 bytes)
    ///
    /// # Returns
    /// * Int value
    fn read_int(&mut self) -> i32;

    /// Read a long (8 bytes)
    ///
    /// # Returns
    /// * Long value
    fn read_long(&mut self) -> i64;

    /// Read a double (8 bytes)
    ///
    /// # Returns
    /// * Double value
    fn read_double(&mut self) -> f64;

    /// Read fully into a byte array
    ///
    /// # Arguments
    /// * `buf` - Byte array to fill
    fn read_fully(&mut self, buf: &mut [u8]);

    /// Read fully into a portion of a byte array
    ///
    /// # Arguments
    /// * `buf` - Byte array to fill
    /// * `off` - Start offset into the byte array
    /// * `len` - Number of bytes to read
    fn read_fully_slice(&mut self, buf: &mut [u8], off: usize, len: usize);

    /// Usually acts the same as `read_fully_slice`, but for an encrypted stream
    /// the raw (unencrypted) data is filled
    ///
    /// # Arguments
    /// * `buf` - Byte array to receive the bytes
    /// * `off` - Start offset into the byte array
    /// * `len` - Number of bytes to fill
    fn read_plain(&mut self, buf: &mut [u8], off: usize, len: usize);
}
