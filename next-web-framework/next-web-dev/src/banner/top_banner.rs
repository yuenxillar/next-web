use crate::util::date_time_util::LocalDateTimeUtil;

pub static DEFAULT_TOP_BANNER: &str = r#"
_   _           _        _____      _                  
| \ | | _____  _| |_     |  ___|   _| |_ _   _ _ __ ___ 
|  \| |/ _ \ \/ / __|____| |_ | | | | __| | | | '__/ _ \
| |\  |  __/>  <| ||_____|  _|| |_| | |_| |_| | | |  __/
|_| \_|\___/_/\_\\__|    |_|   \__,_|\__|\__,_|_|  \___|
"#;

#[derive(Debug)]
pub struct TopBanner;

impl TopBanner {
    pub fn show(banner: &str) {
        print!("{}", banner);
        // print!("\n\t\tversion: {}\n\t\tcontent: {}\n\t\tlink: {}\n", version, content, link);
        println!("\nApplication Running to {}", LocalDateTimeUtil::now());
    }
}
