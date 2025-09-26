extern crate proc_macro;

use crate::data::builder::impl_macro_builder;
use crate::data::constructor::impl_macro_required_args_constructor;
use crate::data::field_name::impl_macro_field_name;
use crate::data::get_set::impl_macro_get_set;
use crate::web::retry::impl_macro_retry;
use data::desensitized::impl_macro_desensitized;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::ItemFn;

mod web;
mod common;
mod data;
mod database;
mod singleton;
mod util;

#[proc_macro_derive(GetSet, attributes(get_set))]
pub fn get_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_get_set(&input)
}

#[proc_macro_derive(FieldName)]
pub fn field_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_field_name(&input)
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_builder(&input)
}

#[proc_macro_derive(RequiredArgsConstructor, attributes(constructor))]
pub fn required_args_constructor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_required_args_constructor(&input)
}

#[proc_macro_derive(Desensitized, attributes(de))]
pub fn desensitized(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_macro_desensitized(&input)
}



// web

#[doc = ""]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn RequestMapping(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::routing::with_method(None, args, input)
}

macro_rules! method_macro {
    ($method:ident, $variant:ident) => {
        
        #[doc = ""]
        #[proc_macro_attribute]
        #[allow(non_snake_case)]
        pub fn $method(args: TokenStream, input: TokenStream) -> TokenStream {
            crate::web::routing::with_method(Some(crate::web::routing::Method::$variant), args, input)
        }
    };
}

method_macro!(GetMapping,       Get);
method_macro!(PostMapping,      Post);
method_macro!(PutMapping,       Put);
method_macro!(DeleteMapping,    Delete);
method_macro!(PatchMapping,     Patch);
method_macro!(AnyMapping,       Any);

#[doc = "A procedural macro attribute for defining scheduled tasks.\n\n\
         This attribute can be applied to a function to register it as a scheduled job\n\
         with configurable timing behavior. It supports three scheduling modes:\n\n\
         - **Cron-based scheduling**: via the `cron` parameter (e.g., `\"0 0 2 * * *\"`).\n\
         - **Fixed-rate execution**: via the `fixed_rate` parameter (executes repeatedly at fixed intervals).\n\
         - **One-shot execution**: when `one_shot = true`, the task runs once after an optional `initial_delay`.\n\n\
         # Parameters\n\n\
         - `cron`: A cron expression in 6-field format (seconds, minutes, hours, day-of-month, month, day-of-week). \n\
           Mutually exclusive with `fixed_rate`.\n\
         - `fixed_rate`: Interval between executions (as a positive integer literal). \n\
           Mutually exclusive with `cron`.\n\
         - `initial_delay`: Delay before the first execution (in units specified by `time_unit`).\n\
         - `timezone`: IANA time zone ID (e.g., `\"Asia/Shanghai\"`, `\"UTC\"`). \n\
           If empty or omitted, the scheduler's default time zone is used.\n\
         - `time_unit`: Time unit for `fixed_rate` and `initial_delay` (e.g., `\"ms\"`, `\"s\"`, `\"m\"`). \n\
           Interpretation depends on the underlying scheduler.\n\
         - `one_shot`: If `true`, the task runs exactly once (typically after `initial_delay`). \n\
           In this mode, `cron` and `fixed_rate` are ignored.\n\n\
         # Examples\n\n\
         ```rust\n\
         #[Scheduled(cron = \"0 0 3 * * *\", timezone = \"UTC\")]\n\
         fn daily_cleanup() { /* ... */ }\n\n\
         #[Scheduled(fixed_rate = 30, time_unit = \"s\", initial_delay = 5)]\n\
         fn heartbeat() { /* ... */ }\n\n\
         #[Scheduled(one_shot = true, initial_delay = 10, time_unit = \"s\")]\n\
         fn delayed_init() { /* ... */ }\n\
         ```"]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Scheduled(args: TokenStream, input: TokenStream) -> TokenStream {
    crate::web::scheduled::impl_macro_scheduled(args, input)
}

#[doc = ""]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Retryable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    impl_macro_retry(attr, item)
}