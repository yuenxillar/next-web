use core::str;
use std::net::SocketAddr;

use crate::service::route_service::RouteWork;

#[derive(Clone)]
pub struct JingYueService {
    name: String,
    addr: SocketAddr,
}

impl Default for JingYueService {
    fn default() -> Self {
        Self {
            name: Default::default(),
            addr: SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                3000,
            ),
        }
    }
}
impl JingYueService {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}

impl JingYueService {
    pub async fn select(
        &self,
        _service_name: &str,
        route_work: &RouteWork,
    ) -> Option<JingYueService> {
        Some(JingYueService::default())
    }
}
