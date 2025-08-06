use next_web_core::DynClone;



pub trait ChatModel: DynClone + Send + Sync{
    
}

next_web_core::clone_trait_object!(ChatModel);