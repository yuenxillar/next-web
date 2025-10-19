use axum::{extract::Request, response::Response};



pub trait HttpFirewall
where 
Self: Send + Sync
{
	/// Provides the request object which will be passed through the filter chain.
	/// RequestRejectedError if the request should be rejected immediately
	/// 
	// fn get_firewalled_request(&self, request: &mut Request) -> Result<FirewalledRequest, RequestRejectedError>;

	
	/// Provides the response which will be passed through the filter chain.
	/// esponse the original response
	/// return either the original response or a replacement/wrapper.
	/// 
	fn get_firewalled_response(&self, response: Response) -> Response;
    
}