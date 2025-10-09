use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq, Eq, Copy)]
pub enum HttpMethod {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Trace
}


impl ToString for HttpMethod {
    fn to_string(&self) -> String {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Head => "HEAD",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Trace => "TRACE",
        }.to_string()
    }
}
impl FromStr for HttpMethod {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let method = s.trim().to_uppercase();
        match method.as_str() {
            "GET" => Ok(HttpMethod::Get),
            "HEAD" => Ok(HttpMethod::Head),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "PATCH" => Ok(HttpMethod::Patch),
            "DELETE" => Ok(HttpMethod::Delete),
            "OPTIONS" => Ok(HttpMethod::Options),
            "TRACE" => Ok(HttpMethod::Trace),
            _ => Err("Invalid HTTP method")
        }
    }
}