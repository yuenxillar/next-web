use crate::model::model_description::ModelDescription;



#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatModel {
    Chat,
    Reasoner,
}


impl ModelDescription for ChatModel {
    fn get_name(&self) -> &str {
        match self {
            ChatModel::Chat => "deepseek-chat",
            ChatModel::Reasoner => "deepseek-reasoner",
        }
    }
}