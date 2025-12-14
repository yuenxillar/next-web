use std::fmt;

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

/// A utility class for working with generic record properties
pub struct GenericRecordUtil;

impl GenericRecordUtil {
    /// Creates a generic properties map with a single key-value pair
    pub fn get_generic_properties_1(val1: &str, sup1: AnyValue) -> IndexMap<String, AnyValue> {
        let mut map = IndexMap::new();
        map.insert(val1.to_string(), sup1);
        map
    }

    /// Creates a generic properties map with two key-value pairs
    pub fn get_generic_properties_2(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1, sup1, val2, sup2, None, None, None, None, None, None, None, None, None, None,
            None, None, None, None,
        )
    }

    /// Creates a generic properties map with three key-value pairs
    pub fn get_generic_properties3(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: &str,
        sup3: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1,
            sup1,
            val2,
            sup2,
            Some(val3),
            Some(sup3),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Creates a generic properties map with four key-value pairs
    pub fn get_generic_properties4(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: &str,
        sup3: AnyValue,
        val4: &str,
        sup4: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1,
            sup1,
            val2,
            sup2,
            Some(val3),
            Some(sup3),
            Some(val4),
            Some(sup4),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Creates a generic properties map with five key-value pairs
    pub fn get_generic_properties5(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: &str,
        sup3: AnyValue,
        val4: &str,
        sup4: AnyValue,
        val5: &str,
        sup5: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1,
            sup1,
            val2,
            sup2,
            Some(val3),
            Some(sup3),
            Some(val4),
            Some(sup4),
            Some(val5),
            Some(sup5),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Creates a generic properties map with six key-value pairs
    pub fn get_generic_properties6(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: &str,
        sup3: AnyValue,
        val4: &str,
        sup4: AnyValue,
        val5: &str,
        sup5: AnyValue,
        val6: &str,
        sup6: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1,
            sup1,
            val2,
            sup2,
            Some(val3),
            Some(sup3),
            Some(val4),
            Some(sup4),
            Some(val5),
            Some(sup5),
            Some(val6),
            Some(sup6),
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Creates a generic properties map with seven key-value pairs
    pub fn get_generic_properties7(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: &str,
        sup3: AnyValue,
        val4: &str,
        sup4: AnyValue,
        val5: &str,
        sup5: AnyValue,
        val6: &str,
        sup6: AnyValue,
        val7: &str,
        sup7: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1,
            sup1,
            val2,
            sup2,
            Some(val3),
            Some(sup3),
            Some(val4),
            Some(sup4),
            Some(val5),
            Some(sup5),
            Some(val6),
            Some(sup6),
            Some(val7),
            Some(sup7),
            None,
            None,
            None,
            None,
        )
    }

    /// Creates a generic properties map with eight key-value pairs
    pub fn get_generic_properties8(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: &str,
        sup3: AnyValue,
        val4: &str,
        sup4: AnyValue,
        val5: &str,
        sup5: AnyValue,
        val6: &str,
        sup6: AnyValue,
        val7: &str,
        sup7: AnyValue,
        val8: &str,
        sup8: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1,
            sup1,
            val2,
            sup2,
            Some(val3),
            Some(sup3),
            Some(val4),
            Some(sup4),
            Some(val5),
            Some(sup5),
            Some(val6),
            Some(sup6),
            Some(val7),
            Some(sup7),
            Some(val8),
            Some(sup8),
            None,
            None,
        )
    }

    /// Creates a generic properties map with nine key-value pairs
    pub fn get_generic_properties9(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: &str,
        sup3: AnyValue,
        val4: &str,
        sup4: AnyValue,
        val5: &str,
        sup5: AnyValue,
        val6: &str,
        sup6: AnyValue,
        val7: &str,
        sup7: AnyValue,
        val8: &str,
        sup8: AnyValue,
        val9: &str,
        sup9: AnyValue,
    ) -> IndexMap<String, AnyValue> {
        Self::get_generic_properties_extended(
            val1,
            sup1,
            val2,
            sup2,
            Some(val3),
            Some(sup3),
            Some(val4),
            Some(sup4),
            Some(val5),
            Some(sup5),
            Some(val6),
            Some(sup6),
            Some(val7),
            Some(sup7),
            Some(val8),
            Some(sup8),
            Some(val9),
            Some(sup9),
        )
    }

    /// Extended method for creating generic properties map with up to 9 key-value pairs
    fn get_generic_properties_extended(
        val1: &str,
        sup1: AnyValue,
        val2: &str,
        sup2: AnyValue,
        val3: Option<&str>,
        sup3: Option<AnyValue>,
        val4: Option<&str>,
        sup4: Option<AnyValue>,
        val5: Option<&str>,
        sup5: Option<AnyValue>,
        val6: Option<&str>,
        sup6: Option<AnyValue>,
        val7: Option<&str>,
        sup7: Option<AnyValue>,
        val8: Option<&str>,
        sup8: Option<AnyValue>,
        val9: Option<&str>,
        sup9: Option<AnyValue>,
    ) -> IndexMap<String, AnyValue> {
        let mut map = IndexMap::new();

        // Process each pair
        let pairs = [
            (val1, sup1),
            (val2, sup2),
            (val3, sup3),
            (val4, sup4),
            (val5, sup5),
            (val6, sup6),
            (val7, sup7),
            (val8, sup8),
            (val9, sup9),
        ];

        for (key, supplier) in pairs {
            if let Some(k) = key {
                if k == "base" {
                    if let Some(s) = supplier {
                        let base_map = s();
                        // In Rust, we need type-safe handling here
                        // This would require a more sophisticated approach
                    }
                } else if let Some(s) = supplier {
                    map.insert(k.to_string(), s);
                }
            }
        }

        map
    }

    /// Creates a safe enum supplier that returns an enum value based on ordinal
    pub fn safe_enum<T: Copy + 'static>(
        values: &'static [T],
        ordinal: Box<dyn Fn() -> i32>,
    ) -> Box<dyn Fn() -> Option<T>> {
        Box::new(move || {
            let ord = ordinal();
            if ord >= 0 && (ord as usize) < values.len() {
                Some(values[ord as usize])
            } else {
                None
            }
        })
    }

    /// Creates a safe enum supplier with a default value
    pub fn safe_enum_with_default<T: Copy + 'static>(
        values: &'static [T],
        ordinal: Box<dyn Fn() -> i32>,
        default_val: T,
    ) -> Box<dyn Fn() -> T> {
        Box::new(move || {
            let ord = ordinal();
            if ord >= 0 && (ord as usize) < values.len() {
                values[ord as usize]
            } else {
                default_val
            }
        })
    }
}

/// Represents annotated flags with descriptions
pub struct AnnotatedFlag {
    value: Box<dyn Fn() -> i32>,
    masks: IndexMap<i32, String>,
    exact_match: bool,
}

impl AnnotatedFlag {
    /// Creates a new AnnotatedFlag
    pub fn new(
        value: Box<dyn Fn() -> i32>,
        masks: &[i32],
        names: &[&str],
        exact_match: bool,
    ) -> Self {
        assert_eq!(
            masks.len(),
            names.len(),
            "Masks and names must have the same length"
        );

        let mut mask_map = IndexMap::new();
        for i in 0..masks.len() {
            mask_map.insert(masks[i], names[i].to_string());
        }

        AnnotatedFlag {
            value,
            masks: mask_map,
            exact_match,
        }
    }

    /// Gets the description of the flags
    pub fn get_description(&self) -> String {
        let val = (self.value)();
        let mut descriptions = Vec::new();

        for (mask, name) in &self.masks {
            if self.exact_match && val == *mask {
                descriptions.push(name.clone());
            } else if !self.exact_match && (val & mask) == *mask {
                descriptions.push(name.clone());
            }
        }

        descriptions.join(" | ")
    }
}

impl fmt::Display for AnnotatedFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_description())
    }
}
