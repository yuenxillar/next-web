
// 测试基本功能
#[cfg(test)]
mod test {
    use rudi::{Properties, SingleOwner};
    use serde::Deserialize;

    #[test]
    fn test_properties_macro() {

        // #[Properties(prefix = "test")]
        #[derive(Debug, Clone, Default, Deserialize)]
        struct TestProperties {
            // #[value = "custom_name"]
            field1: Option<String>,
            field2: Option<i32>,
            field3: bool,
        }

        let props = TestProperties::default();
    }

    trait A {
        fn run_type(&self) -> Self;
    }
    // 测试无前缀
    #[test]
    fn test_properties_no_prefix() {
        #[Properties]
        #[SingleOwner(name = "testPrefix", binds = [Self::into_properties])]
        #[derive(Debug, Clone, Default, Deserialize)]
        struct NoPrefix {
            name: String,
            age: i32,
        }

        impl A for NoPrefix {
            fn run_type(&self) -> Self {
                NoPrefix::default()
            }
        }
        let no_prefix = NoPrefix::default();

        // let trait_obj: &dyn A = &no_prefix;
    }

}
