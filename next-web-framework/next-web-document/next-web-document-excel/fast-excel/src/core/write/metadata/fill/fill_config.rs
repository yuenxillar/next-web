use crate::core::enums::write_direction::WriteDirection,

#[derive(Default)]
pub struct FillConfig {
    direction: Option<WriteDirection>,
    force_new_row: Option<bool>,
    auto_style: Option<bool>,
    has_init: Option<bool>,
}
