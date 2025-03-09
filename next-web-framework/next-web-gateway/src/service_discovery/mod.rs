use core::str;
use std::net::SocketAddr;

use crate::route::route_service_manager::RouteWork;

#[derive(Clone)]
pub struct JingYueServiceDiscovery {}

impl JingYueServiceDiscovery {
    pub async fn select(
        &self,
        _service_name: &str,
        route_work: &RouteWork,
    ) -> Option<JingYueService> {
        Some(JingYueService {
            name: "test".into(),
            addr: SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                3000,
            ),
        })
    }
}
#[derive(Clone)]
pub struct JingYueService {
    name: String,
    addr: SocketAddr,
}

impl JingYueService {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}
