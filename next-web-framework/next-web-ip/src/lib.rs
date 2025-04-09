
// 实现来源为 ip2region 的rust版本实现
// gitee 地址：`https://gitee.com/lionsoul/ip2region/tree/master/binding/rust`

pub use self::ip_value::ToUIntIP;
pub use searcher::{search_by_ip, searcher_init};
pub mod  searcher;

mod ip_value;


