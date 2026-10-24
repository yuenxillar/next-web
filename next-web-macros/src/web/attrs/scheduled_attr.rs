use from_attr::FromAttr;
use syn::{LitInt, LitStr};

/// The attributes of `#[scheduled]`.
///
/// # Attributes
///
/// * `cron` - The schedule of the task as a cron expression of six fields
///   (seconds, minutes, hours, day of month, month, day of week), for example
///   `"0 0 3 * * *"`. It excludes `fixed_rate`.
/// * `fixed_rate` - The number of `time_unit`s between two runs. It excludes
///   `cron`.
/// * `initial_delay` - The number of `time_unit`s the task waits before it is
///   run for the first time. Only `one_shot` tasks support it.
/// * `timezone` - The time zone a cron schedule is evaluated in, for example
///   `"Asia/Shanghai"`, `"UTC"` or `"Local"`.
/// * `time_unit` - The unit of `fixed_rate` and `initial_delay`, one of `ns`,
///   `us`, `ms`, `s`, `m`, `h` or `d` (the long names are supported as well).
///   Milliseconds are the default.
/// * `one_shot` - Runs the task once, after `initial_delay`.
/// * `name` - The key the task is registered under. It defaults to the path of
///   the function, which is what lets a persisted schedule survive a restart.
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
    pub name: Option<LitStr>,
}
