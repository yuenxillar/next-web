use super::binary_error_classifier::BinaryErrorClassifier;



#[derive(Clone, Default)]
pub struct BinaryErrorClassifierBuilder {

}


impl BinaryErrorClassifierBuilder {
    

    pub fn build(self) -> BinaryErrorClassifier {
        BinaryErrorClassifier::default_classifier()
    }
}