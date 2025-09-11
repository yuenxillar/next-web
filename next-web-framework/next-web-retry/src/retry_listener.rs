
pub trait RetryListener where
    Self: Send + Sync,{
    
}


pub struct DefaultRetryListener {

}

impl RetryListener for DefaultRetryListener {
    
}