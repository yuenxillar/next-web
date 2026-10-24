pub mod array_initial_ptg;
pub mod array_ptg;
pub mod base_function_ptg;
pub mod err_ptg;
pub mod func_ptg;
pub mod name_ptg;
pub mod name_x_ptg;
pub mod operand_ptg;
pub mod operation_ptg;
pub mod scalar_constant_ptg;

use std::any::Any;

use next_web_core::error::BoxError;

use crate::core::poi::{
    ss::formula::ptg::{
        array_initial_ptg::ArrayInitialPtg, array_ptg::ArrayPtg, err_ptg::ErrPtg, name_ptg::NamePtg,
    },
    util::{
        little_endian_byte_array_output_stream::LittleEndianByteArrayOutputStream,
        little_endian_input::LittleEndianInput, little_endian_output::LittleEndianOutput,
    },
};

#[derive(Debug, Clone)]
pub struct Ptg {
    ptg_class: u8,
}

impl Ptg {
    /// Reference class
    pub const CLASS_REF: u8 = 0x00;
    /// Value class
    pub const CLASS_VALUE: u8 = 0x20;
    /// Array class
    pub const CLASS_ARRAY: u8 = 0x40;

    /// Reads size bytes from the input stream to create an array of Ptgs.
    /// Extra data (beyond size) may be read if any ArrayPtgs are present.
    pub fn read_tokens(
        size: usize,
        input: &mut dyn LittleEndianInput,
        ext: &dyn PtgExt,
    ) -> Result<Vec<Ptg>, BoxError> {
        let mut tokens = Vec::with_capacity(4 + (size / 2));
        let mut pos = 0;
        let mut has_array_ptgs = false;

        while pos < size {
            let ptg = Self::create_ptg(input, ext)?;
            if (ext as &dyn Any).is::<ArrayInitialPtg>() {
                // ArrayPtg sid
                has_array_ptgs = true;
            }

            pos += ext.get_size();
            tokens.push(ptg);
        }

        if pos != size {
            return Err("Ptg array size mismatch".into());
        }

        if has_array_ptgs {
            // Note: ArrayPtg finishing logic would be implemented here
            // Similar to Java version's finishReading method
            todo!()
        }

        Ok(tokens)
    }

    /// Creates a Ptg from the input stream.
    pub fn create_ptg(
        input: &mut dyn LittleEndianInput,
        ext: &dyn PtgExt,
    ) -> Result<Ptg, BoxError> {
        let id = input.read_byte();

        if id < 0x20 {
            Self::create_base_ptg(id as u8, input)
        } else {
            let mut retval = Self::create_classified_ptg(id as u8, input)?;

            // Set the class based on the ID byte
            if id >= 0x60 {
                retval.set_class(Self::CLASS_ARRAY, ext)?;
            } else if id >= 0x40 {
                retval.set_class(Self::CLASS_VALUE, ext)?;
            } else {
                retval.set_class(Self::CLASS_REF, ext)?;
            }

            Ok(retval)
        }
    }

    /// Creates a classified Ptg (with class bits).
    fn create_classified_ptg(id: u8, input: &mut dyn LittleEndianInput) -> Result<Ptg, BoxError> {
        let base_id = id & 0x1F | 0x20;
        match base_id {
            // 0x20 => Ok(Box::new(ArrayInitialPtg::new(input)?)), // 0x20, 0x40, 0x60
            // 0x21 => Ok(Box::new(FuncPtg::create(input)?)),      // 0x21, 0x41, 0x61
            // 0x22 => Ok(Box::new(FuncVarPtg::create(input)?)),   // 0x22, 0x42, 0x62
            // 0x23 => Ok(Box::new(NamePtg::new(input)?)),         // 0x23, 0x43, 0x63
            // 0x24 => Ok(Box::new(RefPtg::new(input)?)),          // 0x24, 0x44, 0x64
            // 0x25 => Ok(Box::new(AreaPtg::new(input)?)),         // 0x25, 0x45, 0x65
            // 0x26 => Ok(Box::new(MemAreaPtg::new(input)?)),      // 0x26, 0x46, 0x66
            // 0x27 => Ok(Box::new(MemErrPtg::new(input)?)),       // 0x27, 0x47, 0x67
            // 0x29 => Ok(Box::new(MemFuncPtg::new(input)?)),      // 0x29, 0x49, 0x69
            // 0x2A => Ok(Box::new(RefErrorPtg::new(input)?)),     // 0x2a, 0x4a, 0x6a
            // 0x2B => Ok(Box::new(AreaErrPtg::new(input)?)),      // 0x2b, 0x4b, 0x6b
            // 0x2C => Ok(Box::new(RefNPtg::new(input)?)),         // 0x2c, 0x4c, 0x6c
            // 0x2D => Ok(Box::new(AreaNPtg::new(input)?)),        // 0x2d, 0x4d, 0x6d
            // 0x39 => Ok(Box::new(NameXPtg::new(input)?)),        // 0x39, 0x49, 0x79
            // 0x3A => Ok(Box::new(Ref3DPtg::new(input)?)),        // 0x3a, 0x5a, 0x7a
            // 0x3B => Ok(Box::new(Area3DPtg::new(input)?)),       // 0x3b, 0x5b, 0x7b
            // 0x3C => Ok(Box::new(DeletedRef3DPtg::new(input)?)), // 0x3c, 0x5c, 0x7c
            // 0x3D => Ok(Box::new(DeletedArea3DPtg::new(input)?)), // 0x3d, 0x5d, 0x7d
            _ => Err(format!("Unknown Ptg in Formula: 0x{:X} ({})", id, id).into()),
        }
    }

    /// Creates a base Ptg (without class bits).
    fn create_base_ptg(id: u8, input: &mut dyn LittleEndianInput) -> Result<Ptg, BoxError> {
        match id {
            // 0x00 => Ok(Box::new(UnknownPtg::new(id))), // TODO - not a real Ptg
            // 0x01 => Ok(Box::new(ExpPtg::new(input)?)), // 0x01
            // 0x02 => Ok(Box::new(TblPtg::new(input)?)), // 0x02
            // 0x03 => Ok(Box::new(AddPtg::default())),   // 0x03
            // 0x04 => Ok(Box::new(SubtractPtg::default())), // 0x04
            // 0x05 => Ok(Box::new(MultiplyPtg::default())), // 0x05
            // 0x06 => Ok(Box::new(DividePtg::default())), // 0x06
            // 0x07 => Ok(Box::new(PowerPtg::default())), // 0x07
            // 0x08 => Ok(Box::new(ConcatPtg::default())), // 0x08
            // 0x09 => Ok(Box::new(LessThanPtg::default())), // 0x09
            // 0x0A => Ok(Box::new(LessEqualPtg::default())), // 0x0a
            // 0x0B => Ok(Box::new(EqualPtg::default())), // 0x0b
            // 0x0C => Ok(Box::new(GreaterEqualPtg::default())), // 0x0c
            // 0x0D => Ok(Box::new(GreaterThanPtg::default())), // 0x0d
            // 0x0E => Ok(Box::new(NotEqualPtg::default())), // 0x0e
            // 0x0F => Ok(Box::new(IntersectionPtg::default())), // 0x0f
            // 0x10 => Ok(Box::new(UnionPtg::default())), // 0x10
            // 0x11 => Ok(Box::new(RangePtg::default())), // 0x11
            // 0x12 => Ok(Box::new(UnaryPlusPtg::default())), // 0x12
            // 0x13 => Ok(Box::new(UnaryMinusPtg::default())), // 0x13
            // 0x14 => Ok(Box::new(PercentPtg::default())), // 0x14
            // 0x15 => Ok(Box::new(ParenthesisPtg::default())), // 0x15
            // 0x16 => Ok(Box::new(MissingArgPtg::default())), // 0x16
            // 0x17 => Ok(Box::new(StringPtg::new(input)?)), // 0x17
            // 0x19 => Ok(Box::new(AttrPtg::new(input)?)), // 0x19
            // 0x1C => Ok(Box::new(ErrPtg::read(input)?)), // 0x1c
            // 0x1D => Ok(Box::new(BoolPtg::read(input)?)), // 0x1d
            // 0x1E => Ok(Box::new(IntPtg::new(input)?)), // 0x1e
            // 0x1F => Ok(Box::new(NumberPtg::new(input)?)), // 0x1f
            _ => Err(format!("Unexpected base token id ({})", id).into()),
        }
    }

    pub fn set_class(&mut self, the_ptg_class: u8, ext: &dyn PtgExt) -> Result<(), BoxError> {
        if ext.is_base_token() {
            return Err("setClass should not be called on a base token".into());
        }
        self.ptg_class = the_ptg_class;

        Ok(())
    }

    /// Returns the full size taken to encode the specified Ptgs.
    /// This method will return the same result as `get_encoded_size_without_array_data`
    /// if there are no array tokens present.
    pub fn get_encoded_size(ptgs: &[&dyn PtgExt]) -> usize {
        ptgs.iter().map(|ptg| ptg.get_size()).sum()
    }

    /// Used to calculate the value that should be encoded at the start of the encoded Ptg token array.
    /// Returns the size of the encoded Ptg tokens not including any trailing array data.
    pub fn get_encoded_size_without_array_data(ptgs: Vec<&dyn PtgExt>) -> usize {
        let mut size = 0;
        for ptg in ptgs {
            let _size = ptg.get_size();
            if (ptg as &dyn Any).is::<ArrayPtg>() {
                size += ArrayPtg::PLAIN_TOKEN_SIZE;
            } else {
                size += _size;
            }
        }
        size
    }

    /// Writes the ptgs to the data buffer.
    /// The 2 byte encode length field is not written by this method.
    pub fn serialize_ptgs(
        ptgs: Vec<&mut dyn PtgExt>,
        buffer: &mut [u8],
        offset: usize,
    ) -> Result<usize, BoxError> {
        let mut out = LittleEndianByteArrayOutputStream::from_start_offset(buffer, offset)?;
        let mut array_ptgs = None;

        for ptg in ptgs {
            ptg.write(&mut out)?;

            if (ptg as &dyn Any).is::<ArrayPtg>() {
                if array_ptgs.is_none() {
                    array_ptgs = Some(Vec::with_capacity(5));
                }
                array_ptgs.as_mut().map(|s| s.push(ptg));
            }
        }

        // Write array data if present
        if let Some(ptgs) = array_ptgs {
            for ptg in ptgs {
                if let Some(array_ptg) = (ptg as &mut dyn Any).downcast_mut::<ArrayPtg>() {
                    array_ptg.write_token_value_bytes(&mut out);
                }
            }
        }

        Ok(out.get_write_index() - offset)
    }

    /// # Returns
    /// the 'operand class' (REF/VALUE/ARRAY) for this Ptg
    pub fn get_ptg_class(&self) -> u8 {
        self.ptg_class
    }

    /// Debug / diagnostic method to get this token's 'operand class' type.
    ///
    /// # Returns
    /// 'R' for 'reference', 'V' for 'value', 'A' for 'array' and '.' for base tokens
    pub fn get_rvatype(&self, ext: &dyn PtgExt) -> char {
        if ext.is_base_token() {
            '.'
        } else {
            match self.ptg_class {
                Self::CLASS_REF => 'R',
                Self::CLASS_VALUE => 'V',
                Self::CLASS_ARRAY => 'A',
                _ => panic!("Unknown operand class ({})", self.ptg_class),
            }
        }
    }

    pub fn does_formula_refer_to_deleted_cell(ptgs: &[&dyn PtgExt]) -> bool {
        ptgs.iter().any(|ptg| Self::is_deleted_cell_ref(*ptg))
    }

    pub fn is_deleted_cell_ref(ptg: &dyn PtgExt) -> bool {
        // let any = ptg as &dyn Any;

        // if let Some(ptg) = any.downcast_ref::<ErrPtg>() {
        //     return true;
        // }

        // if any.is::<DeletedArea3DPtg>() {
        //     return true;
        // }
        // if any.is::<DeletedRef3DPtg>() {
        //     return true;
        // }
        // if any.is::<AreaErrPtg>() {
        //     return true;
        // }
        // return any.is::<RefErrorPtg>();
        todo!()
    }
}

impl Default for Ptg {
    fn default() -> Self {
        Ptg {
            ptg_class: Self::CLASS_REF,
        }
    }
}

pub trait PtgExt: Any {
    fn is_base_token(&self) -> bool;

    fn get_size(&self) -> usize;

    fn write(&self, out: &mut dyn LittleEndianOutput) -> std::io::Result<()>;

    fn get_default_operand_class(&self) -> u8;

    fn get_sid(&self) -> i8;

    fn to_formula_string(&self) -> Option<String>;
}
