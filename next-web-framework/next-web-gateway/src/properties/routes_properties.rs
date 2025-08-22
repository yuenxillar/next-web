#[derive(Debug, Clone, serde::Deserialize)]
pub struct RoutesProperties {
    pub id: String,
    pub uri: String,
    pub predicates: Vec<String>,
    pub filters: Vec<String>,
    pub order: Option<i32>,
    pub rate_limiter: Option<u16>,
    pub metadata: Option<RouteMetadata>,
}

impl RoutesProperties {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn predicates(&self) -> &Vec<String> {
        &self.predicates
    }

    pub fn filters(&self) -> &Vec<String> {
        &self.filters
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RouteMetadata {
    pub client: Option<ClientMetadata>,
    pub cors: Option<CorsMetadata>,
}


#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClientMetadata {
    pub connect_timeout: Option<u64>,
    pub read_timeout: Option<u64>,
    pub write_timeout: Option<u64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CorsMetadata {
    pub allow_origin: Option<String>,
    pub allow_methods: Option<String>,
    pub allow_headers: Option<String>,
    pub max_age : Option<u32>,
}
