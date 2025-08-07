


pub trait ModelDescription: Send {
    
    fn get_name(&self) -> &str;

    fn get_description(&self) -> &str { "" }

    fn get_version(&self) -> &str { "" }
}