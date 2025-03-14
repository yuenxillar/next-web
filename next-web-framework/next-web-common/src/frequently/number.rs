

#[derive(Debug, Clone)]
pub struct Number {
    value: u32,
}

impl From<u32> for Number {
    
    fn from(value: u32) -> Self {
        Number { value }
    }
}

impl From<f32> for Number {
    
    fn from(value: f32) -> Self {
        let value = (value * 100.0) as u32;
        Number { value }
    }
}

impl From<f64> for Number {
    
    fn from(value: f64) -> Self {
        let value = (value * 100.0) as u32;
        Number { value }
    }
}

impl From<String> for Number {

    fn from(value: String) -> Self {
        let value = if value.contains(".") {
            value.parse::<f64>().unwrap_or_default() as u32
        }
        else {
            value.parse::<u32>().unwrap_or_default()
        };
        Number { value }
    }
}

impl From<&str> for Number {

    fn from(value: &str) -> Self {
        let value = if value.contains(".") {
            (value.parse::<f64>().unwrap_or_default() * 100.0) as u32
        }
        else {
            value.parse::<u32>().unwrap_or_default()
        };
        Number { value }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u32() {
        let big_decimal = Number::from("132.15");
        assert_eq!(big_decimal.value, 13215);
    }
}