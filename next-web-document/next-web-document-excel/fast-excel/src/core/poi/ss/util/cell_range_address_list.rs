use std::convert::TryFrom;
use std::io::{self};

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

use crate::core::poi::common::usermodel::generic_record::GenericRecord;
use crate::core::poi::ss::util::cell_range_address::CellRangeAddress;
use crate::core::poi::util::little_endian_byte_array_output_stream::LittleEndianByteArrayOutputStream;
use crate::core::poi::util::little_endian_output::LittleEndianOutput;

/// Implementation of the cell range address lists, as described
/// in OpenOffice.org's Excel Documentation: excelfileformat.pdf sec 2.5.14 -
/// 'Cell Range Address List'
///
/// In BIFF8 there is a common way to store absolute cell range address lists in
/// several records (not formulas). A cell range address list consists of a field
/// with the number of ranges and the list of the range addresses. Each cell
/// range address (called an ADDR structure) contains 4 16-bit-values.
#[derive(Debug, Clone)]
pub struct CellRangeAddressList {
    /// List of `CellRangeAddress`es. Each structure represents a cell range
    list: Vec<CellRangeAddress>,
}

impl CellRangeAddressList {
    /// Creates a new empty `CellRangeAddressList`.
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    /// Convenience constructor for creating a `CellRangeAddressList` with a single
    /// `CellRangeAddress`. Other `CellRangeAddress`es may be added later.
    pub fn with_single_range(first_row: u32, last_row: u32, first_col: u32, last_col: u32) -> Self {
        let mut list = Self::new();
        list.add_cell_range_address(first_row, first_col, last_row, last_col);
        list
    }

    /// Get the number of following ADDR structures. The number of this
    /// structures is automatically set when reading an Excel file and/or
    /// increased when you manually add a new ADDR structure.
    ///
    /// # Returns
    /// Number of ADDR structures
    pub fn count_ranges(&self) -> usize {
        self.list.len()
    }

    /// Add a cell range structure.
    ///
    /// # Arguments
    /// * `first_row` - the upper left hand corner's row
    /// * `first_col` - the upper left hand corner's column
    /// * `last_row` - the lower right hand corner's row
    /// * `last_col` - the lower right hand corner's column
    pub fn add_cell_range_address(
        &mut self,
        first_row: u32,
        first_col: u32,
        last_row: u32,
        last_col: u32,
    ) {
        let region = CellRangeAddress::new(first_row, last_row, first_col, last_col).unwrap();
        self.add_cell_range_address_object(region);
    }

    /// Add a `CellRangeAddress` object.
    pub fn add_cell_range_address_object(&mut self, cra: CellRangeAddress) {
        self.list.push(cra);
    }

    /// Remove a cell range at the specified index.
    ///
    /// # Arguments
    /// * `range_index` - the index of the range to remove
    ///
    /// # Returns
    /// The removed `CellRangeAddress`
    ///
    /// # Panics
    /// Panics if the list is empty or if the index is out of bounds.
    pub fn remove(&mut self, range_index: usize) -> CellRangeAddress {
        if self.list.is_empty() {
            panic!("List is empty");
        }
        if range_index >= self.list.len() {
            panic!(
                "Range index ({}) is outside allowable range (0..{})",
                range_index,
                self.list.len() - 1
            );
        }
        self.list.remove(range_index)
    }

    /// Get the `CellRangeAddress` at the given index.
    ///
    /// # Arguments
    /// * `index` - the index of the range to retrieve
    ///
    /// # Returns
    /// The `CellRangeAddress` at the specified index
    pub fn get_cell_range_address(&self, index: usize) -> Option<&CellRangeAddress> {
        self.list.get(index)
    }

    /// Get the total encoded size of this list.
    ///
    /// # Returns
    /// The total size including the initial 2 byte range count
    pub fn get_size(&self) -> usize {
        Self::get_encoded_size(self.list.len())
    }

    /// Get the total size for the specified number of ranges,
    /// including the initial 2 byte range count.
    ///
    /// # Arguments
    /// * `number_of_ranges` - the number of ranges
    ///
    /// # Returns
    /// The encoded size
    pub fn get_encoded_size(number_of_ranges: usize) -> usize {
        2 + CellRangeAddress::get_encoded_size(number_of_ranges)
    }

    /// Serialize the list to a byte array.
    ///
    /// # Arguments
    /// * `offset` - starting offset in the data array
    /// * `data` - the byte array to write to
    ///
    /// # Returns
    /// Total number of bytes written
    pub fn serialize_to_array(&self, offset: usize, data: &mut [u8]) -> io::Result<usize> {
        let total_size = self.get_size();
        if offset + total_size > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Buffer too small for serialization",
            ));
        }

        let mut out = LittleEndianByteArrayOutputStream::new(data, offset, total_size)
            .map_err(|s| io::Error::new(io::ErrorKind::Other, s))?;
        self.serialize(&mut out)?;

        Ok(total_size)
    }

    /// Serialize the list to a little-endian output stream.
    ///
    /// # Arguments
    /// * `out` - the output stream
    pub fn serialize(&self, out: &mut dyn LittleEndianOutput) -> io::Result<()> {
        let n_items = u16::try_from(self.list.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Too many ranges for u16"))?;
        out.write_short(n_items)?;

        for region in &self.list {
            region.serialize(out)?;
        }

        Ok(())
    }

    /// Create a copy of this list.
    pub fn copy(&self) -> Self {
        Self {
            list: self.list.iter().map(|region| region.copy()).collect(),
        }
    }

    /// Get all cell range addresses as a slice.
    pub fn get_cell_range_addresses(&self) -> &[CellRangeAddress] {
        &self.list
    }

    /// Get all cell range addresses as a mutable slice.
    pub fn get_cell_range_addresses_mut(&mut self) -> &mut [CellRangeAddress] {
        &mut self.list
    }
}

impl Default for CellRangeAddressList {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericRecord for CellRangeAddressList {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        None
    }

    fn get_generic_children(&self) -> Option<Vec<&dyn GenericRecord>> {
        Some(
            self.list
                .iter()
                .map(|region| region as &dyn GenericRecord)
                .collect(),
        )
    }
}
