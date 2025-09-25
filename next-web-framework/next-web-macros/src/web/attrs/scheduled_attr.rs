use from_attr::FromAttr;
use syn::{LitInt, LitStr};


#[derive(FromAttr)]
#[attribute(idents = [find])]
pub struct ScheduledAttr {
    #[attribute(conflicts = [fixed_rate])]
    pub cron: Option<LitStr>,
    #[attribute(conflicts = [cron])]
    pub fixed_rate: Option<LitInt>,
    pub initial_delay: Option<LitInt>,
    
    pub timezone: Option<LitStr>,
    pub time_unit: Option<LitStr>,

    pub one_shot: bool,
}