use from_attr::FromAttr;
use syn::{LitInt, LitStr};


#[derive(FromAttr)]
#[attribute(idents = [find])]
pub struct ScheduledAttr {
    pub cron: Option<LitStr>,
    #[attribute(conflicts = [cron])]
    pub fixed_rate: Option<LitInt>,
    pub initial_delay: Option<LitInt>,
} 