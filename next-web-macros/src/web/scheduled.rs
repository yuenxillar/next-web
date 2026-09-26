use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{FnArg, Ident, Pat, PatType, ReturnType, Type, Visibility, spanned::Spanned};

use super::attrs::scheduled_attr::ScheduledAttr;

pub(crate) fn impl_macro_scheduled(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = syn::parse_macro_input!(item as syn::ItemFn);

    let attributes = match ScheduledAttr::from_tokens(attr.into()) {
        Ok(attributes) => attributes,
        Err(error) => return error.to_compile_error().into(),
    };

    match ScheduledTask::new(item_fn, attributes) {
        Ok(task) => task.expand().into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// The function a `#[scheduled]` attribute is applied to.
#[derive(Debug)]
struct ScheduledFunction {
    /// The name of the function, which the names of the generated items are
    /// derived from.
    name: Ident,
    /// The visibility of the function, which the generated items inherit so
    /// that a private task is registered by a private item.
    visibility: Visibility,
    /// Whether the function is asynchronous.
    is_async: bool,
    /// Whether the function returns a `Result`, which the task reports as its
    /// failure.
    returns_result: bool,
    /// The dependencies the function takes, in the order it takes them.
    parameters: Vec<ScheduledParameter>,
    /// The function itself, without the attributes the macro consumes.
    item: syn::ItemFn,
}

/// A dependency of a scheduled function.
#[derive(Debug)]
struct ScheduledParameter {
    /// The name of the parameter, which is also the name of the field the
    /// resolved dependency is stored in.
    name: Ident,
    /// The type of the value that is resolved and stored: the type of the
    /// parameter itself, or the type a reference parameter points to.
    stored_type: Type,
    /// Whether the function takes the dependency by reference.
    by_reference: bool,
    /// Whether the parameter declares that the dependency is resolved by its
    /// name instead of by its type.
    find_by_name: bool,
}

/// A `#[scheduled]` function and the schedule it runs on.
struct ScheduledTask {
    function: ScheduledFunction,
    /// The schedule the task runs on.
    schedule: TokenStream2,
    /// The key the task is registered and persisted under.
    task_key: TokenStream2,
}

impl ScheduledTask {
    /// Reads the function and its attributes, and reports the errors that are
    /// known when the application is compiled.
    fn new(
        item_fn: syn::ItemFn,
        ScheduledAttr {
            cron,
            fixed_rate,
            initial_delay,
            timezone,
            time_unit,
            one_shot,
            name,
        }: ScheduledAttr,
    ) -> syn::Result<Self> {
        let function = ScheduledFunction::new(item_fn)?;

        let schedule = Self::schedule(
            &function,
            cron,
            fixed_rate,
            initial_delay,
            timezone,
            time_unit,
            one_shot,
        )?;

        let task_key = match name {
            Some(name) if name.value().trim().is_empty() => {
                return Err(syn::Error::new(name.span(), "name must not be empty"));
            }
            Some(name) => quote! { #name },
            // The path of the function identifies the task across restarts,
            // which is what lets a persisted schedule be restored.
            None => {
                let name = &function.name;
                quote! { ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#name)) }
            }
        };

        Ok(Self {
            function,
            schedule,
            task_key,
        })
    }

    /// Builds the schedule of the task, and reports the attributes that cannot
    /// be combined.
    fn schedule(
        function: &ScheduledFunction,
        cron: Option<syn::LitStr>,
        fixed_rate: Option<syn::LitInt>,
        initial_delay: Option<syn::LitInt>,
        timezone: Option<syn::LitStr>,
        time_unit: Option<syn::LitStr>,
        one_shot: bool,
    ) -> syn::Result<TokenStream2> {
        let time_unit = match time_unit {
            Some(time_unit) => {
                Self::check_time_unit(&time_unit)?;
                Some(time_unit)
            }
            None => None,
        };

        if one_shot {
            let Some(initial_delay) = initial_delay else {
                return Err(syn::Error::new(
                    function.name.span(),
                    "a one shot task has to declare an `initial_delay`",
                ));
            };

            Self::check_duration(&initial_delay, "initial_delay")?;

            let timezone = quote_optional_literal(timezone);
            let time_unit = quote_optional_literal(time_unit);

            return Ok(quote! {
                ::next_web::scheduling::ScheduleType::OneShot(
                    ::next_web::scheduling::WithArgs {
                        initial_delay: ::core::option::Option::Some(#initial_delay),
                        timezone: #timezone,
                        time_unit: #time_unit,
                        ..::core::default::Default::default()
                    }
                )
            });
        }

        if let Some(initial_delay) = initial_delay {
            // A task that repeats on a fixed rate is scheduled directly by the
            // manager; a delay before its first run is only part of the one
            // shot schedule.
            return Err(syn::Error::new(
                initial_delay.span(),
                "`initial_delay` is only supported by a `one_shot` task",
            ));
        }

        if let Some(cron) = cron {
            Self::check_cron(&cron)?;

            let timezone = quote_optional_literal(timezone);

            return Ok(quote! {
                ::next_web::scheduling::ScheduleType::Cron(
                    ::next_web::scheduling::WithArgs {
                        cron: ::core::option::Option::Some(
                            ::std::string::String::from(#cron),
                        ),
                        timezone: #timezone,
                        ..::core::default::Default::default()
                    }
                )
            });
        }

        let Some(fixed_rate) = fixed_rate else {
            return Err(syn::Error::new(
                function.name.span(),
                "a scheduled task has to declare a `cron` expression or a `fixed_rate`",
            ));
        };

        Self::check_duration(&fixed_rate, "fixed_rate")?;

        let time_unit = quote_optional_literal(time_unit);

        Ok(quote! {
            ::next_web::scheduling::ScheduleType::FixedRate(
                ::next_web::scheduling::WithArgs {
                    fixed_rate: ::core::option::Option::Some(#fixed_rate),
                    time_unit: #time_unit,
                    ..::core::default::Default::default()
                }
            )
        })
    }

    /// Checks that a duration is a whole number of units, and that it is not
    /// zero.
    fn check_duration(duration: &syn::LitInt, name: &str) -> syn::Result<()> {
        let value = duration.base10_parse::<u64>().map_err(|_| {
            syn::Error::new(
                duration.span(),
                format!("{name} has to be a positive whole number"),
            )
        })?;

        if value == 0 {
            return Err(syn::Error::new(
                duration.span(),
                format!("{name} has to be greater than 0"),
            ));
        }

        Ok(())
    }

    /// Checks that the time unit is one the scheduler understands.
    fn check_time_unit(time_unit: &syn::LitStr) -> syn::Result<()> {
        const TIME_UNITS: [&str; 14] = [
            "ns",
            "nanoseconds",
            "us",
            "microseconds",
            "ms",
            "milliseconds",
            "s",
            "seconds",
            "m",
            "minutes",
            "h",
            "hours",
            "d",
            "days",
        ];

        let unit = time_unit.value().trim().to_lowercase();

        if TIME_UNITS.contains(&unit.as_str()) {
            return Ok(());
        }

        Err(syn::Error::new(
            time_unit.span(),
            format!(
                "unknown time unit '{unit}', expected one of {}",
                TIME_UNITS.join(", ")
            ),
        ))
    }

    /// Checks that a cron expression has the six fields the scheduler requires.
    fn check_cron(cron: &syn::LitStr) -> syn::Result<()> {
        let fields = cron.value().split_whitespace().count();

        if fields == 6 {
            return Ok(());
        }

        Err(syn::Error::new(
            cron.span(),
            format!(
                "a cron expression has to have six fields \
                 (seconds, minutes, hours, day of month, month, day of week), found {fields}"
            ),
        ))
    }

    /// Expands the function into the pieces the scheduler registers.
    ///
    /// The function is kept as it is, and the macro adds:
    ///
    /// 1. a registration type that the inventory collects, so that the
    ///    application finds the task without an application listing it,
    /// 2. a task that owns the dependencies of the function, so that the
    ///    persisted scheduler can run the task by its key,
    /// 3. the implementation of the registration trait, which resolves those
    ///    dependencies and builds the registration of the task.
    ///
    /// The generated items are named after the function but are not part of its
    /// API: they are the pieces the framework drives the function with.
    fn expand(self) -> TokenStream2 {
        let Self {
            function:
                ScheduledFunction {
                    name,
                    visibility,
                    is_async,
                    returns_result,
                    parameters,
                    item,
                },
            schedule,
            task_key,
        } = self;

        let registration = format_ident!("__ScheduledRegistration{}", name, span = name.span());
        let task = format_ident!("__ScheduledTask{}", name, span = name.span());

        let fields = parameters.iter().map(|parameter| {
            let field = &parameter.name;
            let ty = &parameter.stored_type;
            quote! { #field: #ty }
        });

        let field_names = parameters.iter().map(|parameter| &parameter.name);

        let resolutions = parameters.iter().map(|parameter| {
            let argument = &parameter.name;
            let ty = &parameter.stored_type;

            if parameter.find_by_name {
                quote! {
                    let #argument =
                        __ctx.resolve_with_name::<#ty>(::core::stringify!(#argument));
                }
            } else {
                quote! {
                    let #argument = __ctx.resolve_with_default_name::<#ty>();
                }
            }
        });

        let arguments = parameters.iter().map(|parameter| {
            let field = &parameter.name;

            if parameter.by_reference {
                quote! { &self.#field }
            } else {
                quote! { self.#field.clone() }
            }
        });

        let await_task = if is_async { quote! { .await } } else { quote! {} };

        let run = if returns_result {
            // A task that fails reports the failure to the scheduler, which
            // logs it and keeps the schedule of the task running.
            quote! {
                #name( #(#arguments),* ) #await_task .map(|_| ()).map_err(|error| {
                    ::std::boxed::Box::new(error) as ::next_web::core::error::BoxError
                })?;
            }
        } else {
            quote! {
                #name( #(#arguments),* ) #await_task;
            }
        };

        quote! {
            #item

            #[doc = "Registers the scheduled task of the function above."]
            #[allow(non_camel_case_types)]
            #visibility struct #registration;

            #[doc(hidden)]
            #[allow(non_camel_case_types, non_snake_case, dead_code)]
            struct #task {
                #(#fields,)*
            }

            #[::next_web::core::async_trait]
            impl ::next_web::scheduling::ScheduledJobHandler
                for #task
            {
                fn id(&self) -> &str {
                    #task_key
                }

                async fn execute(
                    &self,
                    _context: ::next_web::scheduling::JobExecutionContext,
                    _payload: ::core::option::Option<::serde_json::Value>,
                ) -> ::core::result::Result<(), ::next_web::core::error::BoxError> {
                    #run

                    ::core::result::Result::Ok(())
                }
            }

            impl ::next_web::scheduling::SchedulerAutoRegister
                for #registration
            {
                fn register(
                    &self,
                    __ctx: &mut dyn ::next_web::core::ApplicationContext,
                ) -> ::core::result::Result<
                    ::next_web::scheduling::ScheduledJobRegistration,
                    ::next_web::core::error::BoxError,
                > {
                    use ::next_web::context::ApplicationContextExt as _;

                    #(#resolutions)*

                    let __task = #task { #(#field_names,)* };

                    ::core::result::Result::Ok(
                        ::next_web::scheduling::ScheduledJobRegistration::new(
                            #task_key,
                            #schedule,
                            ::std::sync::Arc::new(__task),
                        ),
                    )
                }
            }

            ::next_web::submit_scheduler!(#registration);
        }
    }
}

impl ScheduledFunction {
    /// Reads the function the attribute is applied to.
    fn new(mut item: syn::ItemFn) -> syn::Result<Self> {
        let signature = item.sig.clone();

        for input in item.sig.inputs.iter_mut() {
            if let FnArg::Receiver(receiver) = input {
                return Err(syn::Error::new(
                    receiver.span(),
                    "a scheduled task is a function, remove the `self` parameter",
                ));
            }
        }

        let mut parameters = Vec::with_capacity(signature.inputs.len());
        for input in item.sig.inputs.iter_mut() {
            if let FnArg::Typed(parameter) = input {
                parameters.push(ScheduledParameter::new(parameter)?);
            }
        }

        let returns_result = match &signature.output {
            ReturnType::Default => false,
            ReturnType::Type(_, ty) => match ty.as_ref() {
                Type::Tuple(tuple) if tuple.elems.is_empty() => false,
                Type::Path(path) if is_result(path) => true,
                other => {
                    return Err(syn::Error::new(
                        other.span(),
                        "a scheduled task returns nothing, or a `Result` whose \
                         failure the scheduler logs",
                    ));
                }
            },
        };

        Ok(Self {
            name: signature.ident,
            visibility: item.vis.clone(),
            is_async: signature.asyncness.is_some(),
            returns_result,
            parameters,
            item,
        })
    }
}

impl ScheduledParameter {
    /// Reads the dependency a parameter of the task declares.
    fn new(parameter: &mut PatType) -> syn::Result<Self> {
        let name = match parameter.pat.as_ref() {
            Pat::Ident(ident) => ident.ident.clone(),
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "a scheduled task names the dependencies it takes, use an \
                     identifier as the name of the parameter",
                ));
            }
        };

        let (stored_type, by_reference) = match parameter.ty.as_ref() {
            Type::Reference(reference) => {
                if reference.mutability.is_some() {
                    return Err(syn::Error::new(
                        reference.span(),
                        "a scheduled task cannot take a `&mut` dependency, it \
                         shares the dependencies of the application",
                    ));
                }

                (reference.elem.as_ref().clone(), true)
            }
            ty => (ty.clone(), false),
        };

        let find_by_name = parameter
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("find"));

        // The attribute is consumed by the macro, so the function is emitted
        // without it.
        parameter
            .attrs
            .retain(|attribute| !attribute.path().is_ident("find"));

        Ok(Self {
            name,
            stored_type,
            by_reference,
            find_by_name,
        })
    }
}

/// Whether a type is the `Result` of the standard library.
fn is_result(path: &syn::TypePath) -> bool {
    path.path
        .segments
        .last()
        .map(|segment| segment.ident == "Result")
        .unwrap_or_default()
}

/// Quotes the optional literal of an attribute.
///
/// A schedule declares only the values its attributes provide, so an attribute
/// that is missing becomes `None`.
///
/// # Arguments
///
/// * `literal` - The literal to quote, when the task declared it.
fn quote_optional_literal(literal: Option<syn::LitStr>) -> TokenStream2 {
    match literal {
        Some(literal) => quote! {
            ::core::option::Option::Some(::std::string::String::from(#literal))
        },
        None => quote! { ::core::option::Option::None },
    }
}

#[cfg(test)]
mod tests {
    use from_attr::FromAttr;
    use quote::ToTokens;

    use super::{ScheduledAttr, ScheduledFunction, ScheduledTask, is_result};

    #[test]
    fn a_cron_task_becomes_a_cron_schedule() {
        let item = syn::parse_quote! {
            async fn cleanup() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let schedule = ScheduledTask::schedule(
            &function,
            Some(syn::parse_quote! { "0 0 3 * * *" }),
            None,
            None,
            Some(syn::parse_quote! { "UTC" }),
            None,
            false,
        )
        .unwrap()
        .to_string();

        assert!(schedule.contains("Cron"));
        assert!(schedule.contains("UTC"));
    }

    #[test]
    fn a_fixed_rate_task_becomes_a_fixed_rate_schedule() {
        let item = syn::parse_quote! {
            async fn heartbeat() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let schedule = ScheduledTask::schedule(
            &function,
            None,
            Some(syn::parse_quote! { 30 }),
            None,
            None,
            Some(syn::parse_quote! { "s" }),
            false,
        )
        .unwrap()
        .to_string();

        assert!(schedule.contains("FixedRate"));
        assert!(schedule.contains("30"));
    }

    #[test]
    fn a_one_shot_task_becomes_a_one_shot_schedule() {
        let item = syn::parse_quote! {
            async fn warm_up() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let schedule = ScheduledTask::schedule(
            &function,
            None,
            None,
            Some(syn::parse_quote! { 500 }),
            None,
            Some(syn::parse_quote! { "ms" }),
            true,
        )
        .unwrap()
        .to_string();

        assert!(schedule.contains("OneShot"));
        assert!(schedule.contains("500"));
    }

    #[test]
    fn a_cron_expression_needs_six_fields() {
        let item = syn::parse_quote! {
            async fn cleanup() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let error = ScheduledTask::schedule(
            &function,
            Some(syn::parse_quote! { "0 3 * * *" }),
            None,
            None,
            None,
            None,
            false,
        )
        .unwrap_err();

        assert!(error.to_string().contains("six fields"));
    }

    #[test]
    fn an_unknown_time_unit_is_reported() {
        let item = syn::parse_quote! {
            async fn heartbeat() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let error = ScheduledTask::schedule(
            &function,
            None,
            Some(syn::parse_quote! { 30 }),
            None,
            None,
            Some(syn::parse_quote! { "fortnight" }),
            false,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown time unit"));
    }

    #[test]
    fn an_initial_delay_is_only_supported_by_a_one_shot_task() {
        let item = syn::parse_quote! {
            async fn heartbeat() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let error = ScheduledTask::schedule(
            &function,
            None,
            Some(syn::parse_quote! { 30 }),
            Some(syn::parse_quote! { 5 }),
            None,
            None,
            false,
        )
        .unwrap_err();

        assert!(error.to_string().contains("only supported by a `one_shot`"));
    }

    #[test]
    fn a_task_without_a_schedule_is_reported() {
        let item = syn::parse_quote! {
            async fn heartbeat() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let error = ScheduledTask::schedule(&function, None, None, None, None, None, false)
            .unwrap_err();

        assert!(error.to_string().contains("cron"));
    }

    #[test]
    fn a_duration_of_zero_is_reported() {
        let item = syn::parse_quote! {
            async fn heartbeat() {}
        };

        let function = ScheduledFunction::new(item).unwrap();
        let error = ScheduledTask::schedule(
            &function,
            None,
            Some(syn::parse_quote! { 0 }),
            None,
            None,
            None,
            false,
        )
        .unwrap_err();

        assert!(error.to_string().contains("greater than 0"));
    }

    #[test]
    fn a_method_is_reported() {
        let item = syn::parse_quote! {
            async fn heartbeat(&self) {}
        };

        let error = ScheduledFunction::new(item).unwrap_err();

        assert!(error.to_string().contains("`self`"));
    }

    #[test]
    fn a_return_value_that_is_not_a_result_is_reported() {
        let item = syn::parse_quote! {
            async fn heartbeat() -> u32 { 0 }
        };

        let error = ScheduledFunction::new(item).unwrap_err();

        assert!(error.to_string().contains("returns nothing"));
    }

    #[test]
    fn the_dependencies_of_a_task_are_read() {
        let item = syn::parse_quote! {
            async fn heartbeat(clock: Clock, #[find] named: Named) {}
        };

        let function = ScheduledFunction::new(item).unwrap();

        assert_eq!(function.parameters.len(), 2);
        assert_eq!(function.parameters[0].name.to_string(), "clock");
        assert!(!function.parameters[0].by_reference);
        assert!(!function.parameters[0].find_by_name);
        assert!(function.parameters[1].find_by_name);
    }

    #[test]
    fn a_reference_dependency_is_resolved_by_value() {
        let item = syn::parse_quote! {
            async fn heartbeat(clock: &Clock) {}
        };

        let function = ScheduledFunction::new(item).unwrap();

        assert!(function.parameters[0].by_reference);
        assert_eq!(
            function.parameters[0].stored_type.to_token_stream().to_string(),
            "Clock"
        );
    }

    #[test]
    fn a_result_is_recognized() {
        assert!(is_result(&syn::parse_quote! { Result<(), Error> }));
        assert!(is_result(&syn::parse_quote! {
            std::result::Result<(), Error>
        }));
        assert!(!is_result(&syn::parse_quote! { Vec<u8> }));
    }

    #[test]
    fn the_attribute_reads_the_key_of_a_task() {
        let attributes = ScheduledAttr::from_tokens(
            quote::quote! { fixed_rate = 5, time_unit = "s", name = "cleanup" },
        )
        .unwrap();

        assert_eq!(
            attributes.name.map(|name| name.value()),
            Some(String::from("cleanup"))
        );
    }

    #[test]
    fn cron_and_a_fixed_rate_cannot_be_combined() {
        let attributes = ScheduledAttr::from_tokens(
            quote::quote! { cron = "* * * * * *", fixed_rate = 5 },
        );

        assert!(attributes.is_err());
    }
}
