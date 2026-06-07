

pub trait OncePerRequestFilter {

}


// impl<T>  for T where T: OncePerRequestFilter{
    
//     async fn do_filter_internal(
//         &self,
//         request: &mut dyn HttpRequest,
//         response: &mut dyn HttpResponse,
//         filter_chain: &mut dyn HttpFilterChain,
//     ) {
//         self.do_filter(request, response, filter_chain).await;
//     }
// }