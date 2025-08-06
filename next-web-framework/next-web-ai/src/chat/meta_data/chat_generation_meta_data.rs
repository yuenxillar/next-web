use next_web_core::{clone_trait_object, DynClone};

use crate::model::result_meta_data::ResultMetadata;



pub trait ChatGenerationMetadata: DynClone + ResultMetadata {
    
}

clone_trait_object!(ChatGenerationMetadata);