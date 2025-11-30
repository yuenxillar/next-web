use std::{collections::BTreeMap, iter::Cloned, marker::PhantomData};

use tracing::debug;

use crate::core::{enums::head_kind::HeadKind, metadata::head::Head};

#[derive(Debug, Clone)]
pub struct ExcelHeadProperty<T> {
    head_type: PhantomData<T>,
    /// The types of head
    head_kind: HeadKind,
    /// The number of rows in the line with the most rows
    head_row_number: u32,
    /// Configuration header information
    head_map: BTreeMap<u32, Head>,
}

impl<T> ExcelHeadProperty<T> {
    pub fn new(configuration_holder: ConfigurationHolder, head: &[Vec<String>]) -> Self {
        let mut property = Self {
            head_type: PhantomData,
            head_kind: HeadKind::None,
            head_row_number: 0,
            head_map: BTreeMap::new(),
        };

        if !head.is_empty() {
            let mut head_index: u32 = 0;

            for i in 0..head.len() {
                let head = Head::new(head_index, None, head.get(i).map(Clone::clone), false, true);
                property.head_map.insert(head_index, head);
                head_index += 1;
            }

            property.head_kind = HeadKind::Sting;
        }

        // convert headClazz to head
        property.init_column_properties();

        property.init_head_row_number();
        debug!(
            "The initialization sheet/table 'ExcelHeadProperty' is complete , head kind is {:?}",
            &property.head_kind
        );

        todo!();
        property
    }

    fn init_head_row_number(&mut self) {
        self.head_row_number = 0;

        for head in self.head_map.values() {
            if let Some(list) = head.get_head_name_list() {
                if list.len() > (self.head_row_number as usize) {
                    self.head_row_number = list.len() as u32;
                }
            }
        }

        for head in self.head_map.values_mut() {
            if let Some(list) = head.get_mut_head_name_list() {
                if list.is_empty() {
                    continue;
                }
                if list.len() < (self.head_row_number as usize) {
                    let lack = self.head_row_number - (list.len() as u32);
                    let last = list.len() - 1;
                    for _i in 0..lack {
                        list.push(list.get(last).map(Clone::clone).unwrap_or_default());
                    }
                }
            }
        }
    }

    fn init_column_properties(&mut self) {}

    pub fn has_head(&self) -> bool {
        self.head_kind != HeadKind::None
    }
}
