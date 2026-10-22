//! A throughput matrix of the runtime.
//!
//! Run it with:
//!
//! ```text
//! cargo test -p next-web-state-machine --release --test performance -- --ignored --nocapture
//! ```
//!
//! Every scenario builds the same two state cycle (`1 --100--> 2 --100--> 1`) and
//! measures the events which the machine processes, so the numbers compare the
//! cost of the pipeline under different conditions:
//!
//! - the actions of the machine do nothing, or they await once (`yield_now`),
//! - the runtime is a current thread runtime, or a runtime with four workers,
//! - the events come from one task, or from four tasks which compete for the
//!   single event slot of the machine,
//! - the event is accepted, or no transition matches it,
//! - the transitions are driven by a timer instead of by a sender.
//!
//! The numbers are meant for a comparison between scenarios on one machine, not
//! as an absolute promise.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use next_web_core::messaging::generic_message::GenericMessage;
use next_web_state_machine::BoxedStateAction;
use next_web_state_machine::StateMachine;
use next_web_state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web_state_machine::config::model::configuration_data::ConfigurationData;
use next_web_state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web_state_machine::config::model::state_data::StateData;
use next_web_state_machine::config::model::transition_data::TransitionData;
use next_web_state_machine::config::model::transitions_data::TransitionsData;
use next_web_state_machine::config::state_machine_factory::StateMachineFactory;
use next_web_state_machine::state_context::StateContext;

/// Events per measured run.
const EVENTS: usize = 20_000;

/// Events which are processed before the measurement starts.
const WARMUP: usize = 2_000;

/// Builds a configuration with the defaults.
fn configuration(machine_id: &str) -> ConfigurationData<i32, i32> {
    ConfigurationData::with_parameters(
        false,
        None,
        Vec::new(),
        false,
        None,
        None,
        None,
        None,
        false,
        None,
        Some(machine_id.to_owned()),
        None,
        Vec::new(),
    )
}

/// An action which completes without awaiting.
fn sync_action() -> BoxedStateAction<i32, i32> {
    Arc::new(|_context: &dyn StateContext<i32, i32>| Box::pin(async move { Ok(()) }))
}

/// An action which awaits the scheduler once.
fn async_action() -> BoxedStateAction<i32, i32> {
    Arc::new(|_context: &dyn StateContext<i32, i32>| {
        Box::pin(async move {
            tokio::task::yield_now().await;

            Ok(())
        })
    })
}

/// An action which counts how often it ran.
fn counting_action(counter: Arc<AtomicUsize>) -> BoxedStateAction<i32, i32> {
    Arc::new(move |_context: &dyn StateContext<i32, i32>| {
        let counter = counter.clone();

        Box::pin(async move {
            counter.fetch_add(1, Ordering::Relaxed);

            Ok(())
        })
    })
}

/// Builds a machine which cycles between the states 1 and 2 on the event 100.
fn cycle_machine(
    action: BoxedStateAction<i32, i32>,
    period: Option<u64>,
) -> Arc<dyn StateMachine<i32, i32>> {
    let mut initial = StateData::<i32, i32>::with_initial(1, true);
    initial.set_entry_actions(vec![action.clone()]);

    let mut second = StateData::<i32, i32>::new(2);
    second.set_entry_actions(vec![action]);

    let transitions = match period {
        Some(period) => vec![
            timed_transition(1, 2, period),
            timed_transition(2, 1, period),
        ],
        None => vec![TransitionData::new(1, 2, 100), TransitionData::new(2, 1, 100)],
    };

    let model = DefaultStateMachineModel::new(
        configuration("performance"),
        Some(vec![initial, second].into()),
        Some(TransitionsData::new(transitions)),
    );

    BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None)
        .get_state_machine()
        .expect("the machine builds")
}

/// A transition which fires every period, forever.
fn timed_transition(source: i32, target: i32, period: u64) -> TransitionData<i32, i32> {
    TransitionData::with_all(
        source,
        target,
        None,
        None,
        Some(period),
        Some(0),
        Vec::new(),
        None,
        next_web_state_machine::transition::transition_kind::TransitionKind::External,
        None,
        "timer",
    )
}

/// Sends the events one after another and returns the elapsed time.
async fn send_sequentially(machine: &Arc<dyn StateMachine<i32, i32>>, events: usize) -> Duration {
    let message = Arc::new(GenericMessage::with_payload(100));

    let started = Instant::now();

    for _ in 0..events {
        machine
            .send_event(message.clone())
            .await
            .expect("the event is sent");
    }

    started.elapsed()
}

/// Prints the throughput of a run.
fn report(name: &str, events: usize, elapsed: Duration) {
    let per_event = elapsed.as_nanos() as f64 / events as f64;
    let per_second = events as f64 / elapsed.as_secs_f64();

    println!("{name:<34} {per_event:>8.1} ns/event {per_second:>12.0} events/s");
}

/// The throughput matrix.
#[test]
#[ignore = "a performance run, executed on demand"]
fn performance_matrix() {
    run("sync actions, current thread", sync_action(), true, false);
    run("async actions, current thread", async_action(), true, false);
    run("sync actions, four workers", sync_action(), false, false);
    run("async actions, four workers", async_action(), false, false);
    run("concurrent senders, four tasks", async_action(), false, true);
    denied_events();
    timer_driven();
}

/// Runs one scenario of the matrix.
fn run(name: &str, action: BoxedStateAction<i32, i32>, current_thread: bool, concurrent: bool) {
    let runtime = if current_thread {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("the current thread runtime builds")
    } else {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("the multi thread runtime builds")
    };

    runtime.block_on(async move {
        let machine = cycle_machine(action, None);
        machine.start().await.expect("the machine starts");

        let _ = send_sequentially(&machine, WARMUP).await;

        let elapsed = if concurrent {
            let senders = 4;
            let per_sender = EVENTS / senders;

            let started = Instant::now();

            let mut tasks = Vec::with_capacity(senders);

            for _ in 0..senders {
                let machine = machine.clone();

                tasks.push(tokio::spawn(async move {
                    send_sequentially(&machine, per_sender).await;
                }));
            }

            for task in tasks {
                task.await.expect("the sender finishes");
            }

            started.elapsed()
        } else {
            send_sequentially(&machine, EVENTS).await
        };

        report(name, EVENTS, elapsed);
    });
}

/// Measures the events which no transition accepts.
fn denied_events() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("the current thread runtime builds");

    runtime.block_on(async {
        let machine = cycle_machine(sync_action(), None);
        machine.start().await.expect("the machine starts");

        let message = Arc::new(GenericMessage::with_payload(999));

        for _ in 0..WARMUP {
            let _ = machine.send_event(message.clone()).await;
        }

        let started = Instant::now();

        for _ in 0..EVENTS {
            let _ = machine.send_event(message.clone()).await;
        }

        report("denied event, current thread", EVENTS, started.elapsed());
    });
}

/// Measures the transitions a timer drives and the resolution of the platform
/// timer, which no period can beat.
fn timer_driven() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("the runtime builds");

    runtime.block_on(async {
        // The resolution of the platform timer, measured with 100 sleeps of two
        // milliseconds. On Windows this is usually about 15.6ms, so a period
        // below that is not honoured.
        let started = Instant::now();

        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }

        println!(
            "{:<34} {:>8.2} ms per requested 2ms sleep",
            "platform timer resolution",
            (started.elapsed() / 100).as_secs_f64() * 1_000.0
        );

        let counter = Arc::new(AtomicUsize::new(0));
        let period_millis = 50;
        let machine = cycle_machine(counting_action(counter.clone()), Some(period_millis));

        machine.start().await.expect("the machine starts");

        let window = Duration::from_secs(2);

        tokio::time::sleep(window).await;

        let transitions = counter.load(Ordering::Relaxed);

        if transitions > 0 {
            let achieved = window.as_secs_f64() * 1_000.0 / transitions as f64;

            println!(
                "{:<34} {:>8.0} transitions/s  (requested period {}ms, achieved {:.1} ms)",
                "timer driven",
                transitions as f64 / window.as_secs_f64(),
                period_millis,
                achieved
            );
        }

        machine.stop().await.expect("the machine stops");
    });
}
