use crate::core::poi::util::little_endian_output::LittleEndianOutput;

pub trait DelayableLittleEndianOutput: LittleEndianOutput {
    fn create_delayed_output(&mut self, size: usize) -> impl LittleEndianOutput;
}
