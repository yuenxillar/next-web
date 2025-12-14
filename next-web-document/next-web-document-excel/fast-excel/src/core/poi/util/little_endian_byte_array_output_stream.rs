use std::io::{self, Write};

use crate::core::poi::util::{
    delayable_little_endian_output::DelayableLittleEndianOutput,
    little_endian_output::LittleEndianOutput,
};

/// Adapts a plain byte array to LittleEndianOutput trait
pub struct LittleEndianByteArrayOutputStream<'a> {
    buf: &'a mut [u8],
    write_index: usize,
    end_index: usize,
}

impl<'a> LittleEndianByteArrayOutputStream<'a> {
    /// Creates a new LittleEndianByteArrayOutputStream with the specified buffer,
    /// starting offset, and maximum write length.
    ///
    /// # Arguments
    /// * `buf` - The byte buffer to write to
    /// * `start_offset` - The starting position in the buffer
    /// * `max_write_len` - The maximum number of bytes that can be written
    ///
    /// # Panics
    /// Panics if start_offset is out of bounds or if calculated end index is invalid
    pub fn new(
        buf: &'a mut [u8],
        start_offset: usize,
        max_write_len: usize,
    ) -> Result<Self, String> {
        if start_offset < 0 || start_offset > buf.len() {
            return Err(format!(
                "Specified startOffset ({}) is out of allowable range (0..{})",
                start_offset,
                buf.len()
            ));
        }

        let end_index = start_offset + max_write_len;
        if end_index < start_offset || end_index > buf.len() {
            return Err(format!(
                "calculated end index ({}) is out of allowable range ({}..{})",
                end_index,
                start_offset,
                buf.len()
            ));
        }

        Ok(Self {
            buf,
            write_index: start_offset,
            end_index,
        })
    }

    /// Creates a new LittleEndianByteArrayOutputStream that can write
    /// from start_offset to the end of the buffer.
    pub fn from_start_offset(buf: &'a mut [u8], start_offset: usize) -> Result<Self, String> {
        Self::new(buf, start_offset, buf.len() - start_offset)
    }

    /// Checks if there's enough space to write the specified number of bytes.
    ///
    /// # Panics
    /// Panics if there's insufficient space
    fn check_position(&self, i: usize) -> Result<(), String> {
        if i > self.available() {
            return Err(format!(
                "Buffer overrun: need {} bytes, have {} available",
                i,
                self.available()
            ));
        }
        Ok(())
    }

    /// Returns the number of bytes that can still be written
    fn available(&self) -> usize {
        self.end_index - self.write_index
    }

    /// Returns the current write index (position in the buffer)
    pub fn get_write_index(&self) -> usize {
        self.write_index
    }
}

impl<'a> Write for LittleEndianByteArrayOutputStream<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        self.check_position(len);

        let dest_slice = &mut self.buf[self.write_index..self.write_index + len];
        dest_slice.copy_from_slice(buf);
        self.write_index += len;

        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(()) // No buffering, nothing to flush
    }
}

impl<'a> LittleEndianOutput for LittleEndianByteArrayOutputStream<'a> {
    fn write_byte(&mut self, v: u8) -> io::Result<()> {
        self.check_position(1)
            .map_err(|s| io::Error::new(io::ErrorKind::Other, s));
        self.buf[self.write_index] = v;
        self.write_index += 1;
        Ok(())
    }

    fn write_double(&mut self, v: f64) -> io::Result<()> {
        self.write_long(v.to_bits())
    }

    fn write_int(&mut self, v: u32) -> io::Result<()> {
        self.check_position(4)
            .map_err(|s| io::Error::new(io::ErrorKind::Other, s));

        let bytes = v.to_le_bytes();
        let dest = &mut self.buf[self.write_index..self.write_index + 4];
        dest.copy_from_slice(&bytes);
        self.write_index += 4;

        Ok(())
    }

    fn write_long(&mut self, v: u64) -> io::Result<()> {
        self.check_position(8)
            .map_err(|s| io::Error::new(io::ErrorKind::Other, s));

        let bytes = v.to_le_bytes();
        let dest = &mut self.buf[self.write_index..self.write_index + 8];
        dest.copy_from_slice(&bytes);
        self.write_index += 8;

        Ok(())
    }

    fn write_short(&mut self, v: u16) -> io::Result<()> {
        self.check_position(2)
            .map_err(|s| io::Error::new(io::ErrorKind::Other, s));

        let bytes = v.to_le_bytes();
        let dest = &mut self.buf[self.write_index..self.write_index + 2];
        dest.copy_from_slice(&bytes);
        self.write_index += 2;

        Ok(())
    }

    fn write_bytes(&mut self, b: &[u8]) -> io::Result<()> {
        self.check_position(b.len())
            .map_err(|s| io::Error::new(io::ErrorKind::Other, s));

        let dest = &mut self.buf[self.write_index..self.write_index + b.len()];
        dest.copy_from_slice(b);
        self.write_index += b.len();

        Ok(())
    }

    fn write_bytes_slice(&mut self, b: &[u8], offset: usize, len: usize) -> io::Result<()> {
        self.check_position(len)
            .map_err(|s| io::Error::new(io::ErrorKind::Other, s));

        let dest = &mut self.buf[self.write_index..self.write_index + len];
        dest.copy_from_slice(&b[offset..offset + len]);
        self.write_index += len;

        Ok(())
    }
}

impl<'a> DelayableLittleEndianOutput for LittleEndianByteArrayOutputStream<'a> {
    fn create_delayed_output(&mut self, size: usize) -> impl LittleEndianOutput {
        self.check_position(size);
        let start = self.write_index;
        self.write_index += size;

        // Create a new view into the same buffer for the delayed output
        LittleEndianByteArrayOutputStream::new(&mut self.buf[start..], 0, size).unwrap()
    }
}

// Additional helper implementations
impl<'a> LittleEndianByteArrayOutputStream<'a> {
    /// Returns a slice of the written data
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.write_index]
    }

    /// Returns a mutable slice of the unwritten portion of the buffer
    pub fn remaining_mut(&mut self) -> &mut [u8] {
        &mut self.buf[self.write_index..self.end_index]
    }

    /// Advances the write index by the specified amount
    ///
    /// # Safety
    /// Caller must ensure the buffer has been written to
    pub fn advance(&mut self, n: usize) {
        self.check_position(n);
        self.write_index += n;
    }
}
