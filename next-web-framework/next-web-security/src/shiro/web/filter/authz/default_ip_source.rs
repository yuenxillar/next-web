use crate::web::filter::authz::ip_source::IpSource;

#[derive(Clone, Default)]
pub struct DefaultIpSource;

impl IpSource for DefaultIpSource {
    fn get_authorized_ips(&self) -> Vec<&str> {
        Vec::with_capacity(0)
    }

    fn get_denied_ips(&self) -> Vec<&str> {
        Vec::with_capacity(0)
    }
}
