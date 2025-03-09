pub trait GlobalFilter {
    fn order(&self) -> i32 {
        i32::MAX
    }

    // fn filter(&self, request: &mut HttpRequest, response: &mut HttpResponse) -> Result<bool, Error> {
    //     Ok(true)
    // }
}
