use crate::config::web::{configurers::{authorize_http_requests_configurer::AuthorizeHttpRequestsConfigurer, expression_url_authorization_configurer::ExpressionUrlAuthorizationConfigurer}, http_security_builder::HttpSecurityBuilder, util::matcher::request_matcher::RequestMatcher};



pub struct PermitAllSupport;


impl PermitAllSupport {

    pub fn permit_all<'a, T>(
        http: &mut T,
        urls: impl IntoIterator<Item = &'a str>
    ) 
    where 
    T: HttpSecurityBuilder<T>
    {
         for url in  urls {
			if !url.is_empty() {
				Self::_permit_all(http,  [ExactUrlRequestMatcher::new(url)]);
			}
        }
    }

    fn _permit_all<T, T1, T2>(http: &mut T, request_matchers: T1) 
    where 
    T: HttpSecurityBuilder<T>,
    T1: IntoIterator<Item = T2>,
    T2: RequestMatcher
    {
        let configurer = http.get_configurer::<ExpressionUrlAuthorizationConfigurer>();
        let http_configurer = http.get_configurer::<AuthorizeHttpRequestsConfigurer>();

        let one_configurer_present = configurer.is_none() ^ http_configurer.is_none();

        assert!( one_configurer_present,
            "permit_all only works with either HttpSecurity.authorizeRequests() or HttpSecurity.authorizeHttpRequests 
                Please define one or the other but not both."
        );

        for matcher in request_matchers.into_iter() {
            if let Some(configurer) = configurer {
                configurer.get_registry().add_mapping(
                    0,
                    UrlMapping::new(
                        matcher,
                        
                    )

                )
            }else {
                if let Some(http_configurer) = http_configurer {
                    http_configurer.add_first(
                        matcher,
                        AuthorizeHttpRequestsConfigurer::permit_all_authorization_manager()
                    )
                }
            }
        }

    }
}

#[derive(Clone, Debug)]
pub struct ExactUrlRequestMatcher {
    process_url: Box<str>
}


impl ExactUrlRequestMatcher {
    pub fn new(
        process_url: impl Into<Box<str>>
    ) -> Self {
        Self {  
            process_url: process_url.into()
        }
    }
}

impl RequestMatcher for ExactUrlRequestMatcher {
    fn matches(&self, request: &axum::extract::Request) -> bool {
        todo!()
    }
}