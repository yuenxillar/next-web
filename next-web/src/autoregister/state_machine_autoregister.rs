//! Registration of the state machines of an application.
//!
//! A configuration which implements `StateMachineConfigurer` contributes its
//! machine to the application context, so a component injects it:
//!
//! ```ignore
//! #[state_machine(name = "orderStateMachine")]
//! impl StateMachineConfigurer<OrderState, OrderEvent> for OrderStateMachineConfig { .. }
//!
//! #[component]
//! struct OrderService {
//!     machine: Arc<dyn StateMachine<OrderState, OrderEvent>>,
//! }
//! ```
//!
//! The attribute of the macro crate submits the configuration to the inventory,
//! and [`apply_state_machines`] builds and registers every submitted machine
//! while the application context is configured.

use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use next_web_context::ApplicationContextExt;
use next_web_core::ApplicationContext;
use next_web_core::BoxFuture;
use next_web_core::error::BoxError;
use next_web_state_machine::StateMachine;
use next_web_state_machine::autoconfigure::state_machine_auto_configuration::StateMachineAutoConfiguration;
use next_web_state_machine::config::builders::state_machine_configurer::StateMachineConfigurer;
use next_web_state_machine::persist::state_machine_persister::StateMachinePersister;

/// The bean name of a machine whose attribute names none.
pub const DEFAULT_STATE_MACHINE_NAME: &str = "stateMachine";

/// A configuration which contributes a state machine to the context.
///
/// The macro `#[state_machine]` implements it for the configuration; a hand
/// written implementation is the alternative when a configuration has to be
/// constructed differently.
pub trait StateMachineAutoRegister
where
    Self: Send + Sync + 'static,
{
    /// The type name of the configuration, used in diagnostics.
    fn configurer_name(&self) -> &'static str;

    /// The bean name the machine is registered under.
    fn machine_name(&self) -> &'static str;

    /// Builds the machine and registers it in the given context.
    ///
    /// # Arguments
    /// * `context` - the context which receives the machine
    fn register(&self, context: &mut dyn ApplicationContext) -> Result<(), BoxError>;
}

inventory::collect!(&'static dyn StateMachineAutoRegister);

/// Submits a configuration as a state machine of the application.
///
/// The macro `#[state_machine]` uses it; call it directly when the configuration
/// does not use the attribute.
///
/// # Arguments
/// * `$ty` - a value of a type which implements [`StateMachineAutoRegister`]
#[macro_export]
macro_rules! submit_state_machine {
    ($ty:path) => {
        ::next_web::inventory::submit! {
            &$ty as &dyn ::next_web::autoregister::state_machine_autoregister::StateMachineAutoRegister
        }
    };
}

/// The future of a stop hook.
type StopFuture = BoxFuture<'static, Result<(), BoxError>>;

/// A closure which stops one registered machine.
type StopClosure = Arc<dyn Fn() -> StopFuture + Send + Sync>;

/// The machines the application registered, in registration order.
fn stop_hooks() -> &'static Mutex<Vec<StopClosure>> {
    static HOOKS: OnceLock<Mutex<Vec<StopClosure>>> = OnceLock::new();

    HOOKS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Builds the machine of a configuration and registers it in the context.
///
/// The machine is registered under its bean name and by the type
/// `Arc<dyn StateMachine<S, E>>`, which is what a component injects. When two
/// machines share one state and event type, inject
/// `Vec<(String, Arc<dyn StateMachine<S, E>>)>` or the service instead.
///
/// # Arguments
/// * `context` - the context which receives the machine
/// * `configurer` - the configuration of the machine
/// * `name` - the bean name of the machine
pub fn register_machine<S, E, T>(
    context: &mut dyn ApplicationContext,
    configurer: T,
    name: &'static str,
) -> Result<(), BoxError>
where
    S: Send + Sync + 'static + Clone + Eq + Hash + std::fmt::Debug,
    E: Send + Sync + 'static + Clone + Eq + Hash,
    T: StateMachineConfigurer<S, E> + Clone + 'static,
{
    let machine: Arc<dyn StateMachine<S, E>> = StateMachineAutoConfiguration::build(configurer)?;

    // A machine which a persister stores is written before it stops, so a
    // shutdown does not lose the state the machine reached.
    let persister = context
        .resolve_by_type::<Arc<dyn StateMachinePersister<S, E, String>>>()
        .into_iter()
        .next();

    let machine_for_stop = machine.clone();
    let name_for_stop = String::from(name);

    stop_hooks()
        .lock()
        .expect("the stop hook registry is not poisoned")
        .push(Arc::new(move || {
            let machine = machine_for_stop.clone();
            let persister = persister.clone();
            let name = name_for_stop.clone();

            Box::pin(async move {
                if let Some(persister) = persister {
                    persister.persist(machine.clone(), name).await?;
                }

                machine.stop().await
            })
        }));

    context.insert_singleton_with_name(machine, name);

    Ok(())
}

/// Builds and registers every machine an inventory registration contributed.
///
/// Two configurations which name their machine the same way are an error,
/// because the bean name would be ambiguous.
///
/// # Arguments
/// * `context` - the context which receives the machines
pub async fn apply_state_machines(context: &mut dyn ApplicationContext) -> Result<(), BoxError> {
    let registrations: Vec<&'static dyn StateMachineAutoRegister> =
        inventory::iter::<&'static dyn StateMachineAutoRegister>
            .into_iter()
            .copied()
            .collect();

    check_unique(
        registrations
            .iter()
            .map(|registration| (registration.machine_name(), registration.configurer_name())),
    )?;

    for registration in registrations {
        registration.register(context)?;
    }

    install_shutdown_hook();

    Ok(())
}

/// Stops every machine the application registered.
///
/// A machine which a persister stores is written before it stops.
pub async fn stop_all_registered_machines() -> Result<(), BoxError> {
    let hooks: Vec<StopClosure> = stop_hooks()
        .lock()
        .expect("the stop hook registry is not poisoned")
        .clone();

    let mut first_error: Option<BoxError> = None;

    for stop in hooks {
        if let Err(error) = stop().await
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Installs the hook which stops every registered machine on shutdown.
///
/// The hook is installed once per process; a machine which is built after the
/// hook exists is stopped as well, because the hook reads the registry when it
/// runs.
fn install_shutdown_hook() {
    static INSTALLED: AtomicBool = AtomicBool::new(false);

    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let hook = crate::next_web_application::APPLICATION_SHUTDOWN_HOOK.get_or_init(Default::default);

    hook.add_hook(|| async {
        if let Err(error) = stop_all_registered_machines().await {
            tracing::warn!(%error, "a state machine could not be stopped at shutdown");
        }
    });
}

/// Rejects two configurations which name their machine the same way.
///
/// # Arguments
/// * `entries` - the bean name and the configuration of every machine
fn check_unique<'a>(entries: impl IntoIterator<Item = (&'a str, &'a str)>) -> Result<(), BoxError> {
    let mut names: Vec<(&str, &str)> = Vec::new();

    for (name, configurer) in entries {
        if let Some((_, other)) = names.iter().find(|(registered, _)| *registered == name) {
            return Err(duplicate_name(name, other, configurer));
        }

        names.push((name, configurer));
    }

    Ok(())
}

/// Builds the error of two configurations which share a bean name.
///
/// # Arguments
/// * `name` - the bean name both configurations declare
/// * `first` - the first configuration which uses it
/// * `second` - the configuration which collides with the first one
fn duplicate_name(name: &str, first: &str, second: &str) -> BoxError {
    Box::new(std::io::Error::other(format!(
        "two state machines are registered as \"{name}\": {first} and {second}; \
         pass name = \"..\" to one of them"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_configurations_with_one_name_are_rejected() {
        let error = check_unique([
            (DEFAULT_STATE_MACHINE_NAME, "FirstConfiguration"),
            (DEFAULT_STATE_MACHINE_NAME, "SecondConfiguration"),
        ])
        .expect_err("the duplicate name is an error");

        assert!(error.to_string().contains("two state machines"));
        assert!(error.to_string().contains(DEFAULT_STATE_MACHINE_NAME));
        assert!(error.to_string().contains("FirstConfiguration"));
        assert!(error.to_string().contains("SecondConfiguration"));
    }

    #[test]
    fn distinct_names_pass_the_check() {
        check_unique([
            (DEFAULT_STATE_MACHINE_NAME, "FirstConfiguration"),
            ("orderStateMachine", "SecondConfiguration"),
        ])
        .expect("distinct names are fine");
    }
}
