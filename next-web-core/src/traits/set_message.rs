pub trait SetMessage {
    fn set_message<S: Into<String>>(&mut self, message: S);
}
