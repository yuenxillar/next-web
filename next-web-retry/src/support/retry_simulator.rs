//! Deterministic simulation of retry backoff sequences.
//!
//! A simulator answers calibration questions without paying the wall clock
//! cost of the sleeps it studies: it walks a failing task chain of
//! `max_attempts` attempts and records the sleep that the backoff policy would
//! have requested before every retry. Because every attempt is assumed to fail,
//! the loop performs `max_attempts - 1` backoff steps, and the recorded values
//! are collected into a [`RetrySimulation`].
//!
//! # Design deviation
//!
//! A real policy performs its wait inside
//! [`BackOffPolicy::backoff`](crate::backoff::back_off_policy::BackOffPolicy::backoff),
//! through the [`Sleeper`](crate::backoff::sleeper::Sleeper) it was configured
//! with. The sleeper can only be replaced on a concrete policy before it is
//! shared, so a simulator that receives an already shared
//! `Arc<dyn BackOffPolicy>` cannot redirect the wait; driving that policy would
//! either sleep for real or only measure elapsed time, and in both cases the
//! result would be non-deterministic.
//!
//! This module therefore introduces the [`SimulatedBackOff`] trait: the same
//! accounting of a backoff policy, expressed as "produce the next sleep for
//! this state" instead of "sleep now". The simulator stays fully deterministic,
//! never sleeps, and reports exactly the durations the policy would request.
//!
//! Every policy of this crate can be simulated directly because each exposes
//! its configuration through getters. One behaviour still differs from a real
//! run: the randomised branches draw from the seeded [`SimState`] generator
//! instead of the process wide generator, so that a whole simulation can be
//! replayed value by value.

use std::sync::Arc;

use crate::{
    backoff::{
        exponential_back_off_policy::ExponentialBackOffPolicy,
        fixed_back_off_policy::FixedBackOffPolicy, no_back_off_policy::NoBackOffPolicy,
        uniform_random_back_off_policy::UniformRandomBackOffPolicy,
    },
    error::retry_error::RetryError,
    support::retry_simulation::{RetrySimulation, SleepSequence},
};

/// Seed used by [`SimState`] when none is supplied.
pub const DEFAULT_SEED: u64 = 0x2545_f491_4f6c_dd1d;

/// Mutable state threaded through one simulated task chain.
///
/// A fresh state is created for every simulated sequence, so one chain never
/// observes the state of another. The state carries the growing interval of an
/// exponential policy and a seeded pseudo random generator for policies that
/// pick a random sleep, which keeps every run reproducible.
#[derive(Clone, Debug)]
pub struct SimState {
    interval: u64,
    produced: u32,
    random_state: u64,
}

impl SimState {
    /// Creates a state with an explicit seed for the pseudo random generator.
    ///
    /// A zero seed is replaced by [`DEFAULT_SEED`] because the generator must
    /// never enter the all-zero fixed point.
    pub fn new(seed: u64) -> Self {
        Self {
            interval: 0,
            produced: 0,
            random_state: if seed == 0 { DEFAULT_SEED } else { seed },
        }
    }

    /// Returns the interval an exponential policy will use for the next sleep.
    ///
    /// The value is `0` until the policy implementation stores one.
    pub fn interval(&self) -> u64 {
        self.interval
    }

    /// Stores the interval an exponential policy will use for the next sleep.
    pub fn set_interval(&mut self, interval: u64) {
        self.interval = interval;
    }

    /// Returns how many sleeps this state has produced so far.
    pub fn produced(&self) -> u32 {
        self.produced
    }

    /// Advances the produced-sleep counter.
    ///
    /// [`RetrySimulator`] calls this after every [`SimulatedBackOff::next_sleep`]
    /// call, so implementations can adapt to the position inside the chain.
    pub fn advance(&mut self) {
        self.produced = self.produced.saturating_add(1);
    }

    /// Returns the next value of the seeded pseudo random generator.
    pub fn next_u64(&mut self) -> u64 {
        let mut value = self.random_state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.random_state = value;
        value.wrapping_mul(DEFAULT_SEED)
    }

    /// Returns a pseudo random value inside the inclusive range `min..=max`.
    ///
    /// When `min >= max` the range is degenerate and `min` is returned.
    pub fn next_in_range(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }

        let span = max - min;
        if span == u64::MAX {
            // The inclusive range cannot be widened without overflowing.
            return self.next_u64();
        }

        min + self.next_u64() % (span + 1)
    }

    /// Returns a pseudo random value in `[0, 1)`.
    pub fn next_unit_f64(&mut self) -> f64 {
        // An `f64` mantissa holds 53 significant bits, so the value is drawn
        // from exactly the numbers that range can represent.
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }
}

impl Default for SimState {
    fn default() -> Self {
        Self::new(DEFAULT_SEED)
    }
}

/// Produces the sleep a backoff policy would request next, without sleeping.
///
/// Implementations may read and update the [`SimState`] they are given: the
/// state belongs to a single simulated chain and carries the growing interval
/// of exponential policies as well as the seeded generator used by random
/// policies.
pub trait SimulatedBackOff
where
    Self: Send + Sync,
{
    /// Returns the sleep in milliseconds for the next retry of the chain.
    fn next_sleep(&self, state: &mut SimState) -> u64;
}

/// A policy that never waits produces a zero sleep for every retry.
impl SimulatedBackOff for NoBackOffPolicy {
    fn next_sleep(&self, _state: &mut SimState) -> u64 {
        0
    }
}

/// Simulates a fixed backoff policy from its configured period.
impl SimulatedBackOff for FixedBackOffPolicy {
    fn next_sleep(&self, _state: &mut SimState) -> u64 {
        self.get_back_off_period()
    }
}

/// Simulates a uniform random backoff policy from its configured bounds.
///
/// The bounds are read on every call, so a policy with a dynamic source is
/// simulated with the value that source reports at that moment. The draw is
/// uniform in `[min, max)`, like the policy; a maximum that is not above the
/// minimum is simulated as a fixed wait.
impl SimulatedBackOff for UniformRandomBackOffPolicy {
    fn next_sleep(&self, state: &mut SimState) -> u64 {
        let min_back_off_period = self.get_min_back_off_period();
        let max_back_off_period = self.get_max_back_off_period();

        if max_back_off_period <= min_back_off_period {
            return min_back_off_period;
        }

        // The policy samples the half open range `[min, max)`, which is the
        // same set of whole milliseconds as `[min, max - 1]`.
        state.next_in_range(min_back_off_period, max_back_off_period - 1)
    }
}

/// Simulates an exponential backoff policy from its public configuration.
///
/// The initial interval, the maximum interval and the multiplier are read from
/// the policy, so the generated sleeps follow the same schedule, including the
/// truncating growth of the interval and the cap at the maximum. A policy with
/// jitter enabled keeps that behaviour, with the jitter factor drawn from the
/// seeded [`SimState`] generator instead of the process wide generator.
impl SimulatedBackOff for ExponentialBackOffPolicy {
    fn next_sleep(&self, state: &mut SimState) -> u64 {
        let interval = match state.interval() {
            0 => self.get_initial_interval(),
            interval => interval,
        };

        let max_interval = self.get_max_interval();
        let sleep = if interval >= max_interval {
            max_interval
        } else {
            state.set_interval(next_interval(interval, self.get_multiplier()));
            interval
        };

        if !self.is_with_random() {
            return sleep;
        }

        let multiplier = self.get_multiplier();
        let jitter = 1.0 + state.next_unit_f64() * (multiplier - 1.0);

        ((sleep as f64 * jitter) as u64).min(max_interval)
    }
}

/// Returns the interval that follows `interval` for the given multiplier.
///
/// The fractional part is truncated, which is how the policy advances its own
/// schedule.
fn next_interval(interval: u64, multiplier: f64) -> u64 {
    // The cast saturates, so an overflowing product becomes `u64::MAX` instead
    // of wrapping.
    (interval as f64 * multiplier) as u64
}

/// Runs a failing task chain and collects the sleeps of the backoff policy.
///
/// The simulator never sleeps and never touches a runtime: every call returns
/// the same sleeps for the same configuration and seed, which makes the
/// resulting [`RetrySimulation`] a stable fixture for tests and calibration.
#[derive(Clone)]
pub struct RetrySimulator {
    max_attempts: u16,
    backoff: Arc<dyn SimulatedBackOff>,
    seed: u64,
}

impl RetrySimulator {
    /// Creates a simulator for the given number of attempts and backoff.
    pub fn new(max_attempts: u16, backoff: impl SimulatedBackOff + 'static) -> Self {
        Self::from_arc(max_attempts, Arc::new(backoff))
    }

    /// Creates a simulator from an already shared backoff.
    pub fn from_arc(max_attempts: u16, backoff: Arc<dyn SimulatedBackOff>) -> Self {
        Self {
            max_attempts,
            backoff,
            seed: DEFAULT_SEED,
        }
    }

    /// Creates a simulator for a policy that always waits for the same period.
    pub fn fixed(max_attempts: u16, back_off_period: u64) -> Self {
        let mut policy = FixedBackOffPolicy::new();
        policy.set_back_off_period(back_off_period);

        Self::new(max_attempts, policy)
    }

    /// Creates a simulator for a policy that waits for a random period.
    pub fn uniform_random(
        max_attempts: u16,
        min_back_off_period: u64,
        max_back_off_period: u64,
    ) -> Self {
        let mut policy = UniformRandomBackOffPolicy::new();
        policy.set_min_back_off_period(min_back_off_period);
        policy.set_max_back_off_period(max_back_off_period);

        Self::new(max_attempts, policy)
    }

    /// Creates a simulator for an exponential backoff policy.
    pub fn exponential(max_attempts: u16, policy: ExponentialBackOffPolicy) -> Self {
        Self::new(max_attempts, policy)
    }

    /// Replaces the seed used by the first simulated sequence.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Returns the number of attempts of the simulated chain.
    pub fn get_max_attempts(&self) -> u16 {
        self.max_attempts
    }

    /// Runs a single failing task chain and returns its sleeps.
    ///
    /// The chain performs `max_attempts - 1` backoff steps, because the first
    /// attempt is made without any sleep; a chain with fewer than two attempts
    /// produces an empty sequence.
    pub fn execute_single_simulation(&self) -> SleepSequence {
        self.simulation_with_seed(self.seed)
    }

    /// Runs a single failing task chain and returns the recorded simulation.
    ///
    /// The returned [`RetrySimulation`] holds one sequence. The `Result` and
    /// the `async` signature match the shape a policy driven simulator would
    /// need; the deterministic simulation itself cannot fail and always
    /// returns `Ok`.
    pub async fn execute(&self) -> Result<RetrySimulation, RetryError> {
        let mut simulation = RetrySimulation::new();
        simulation.add(self.execute_single_simulation());

        Ok(simulation)
    }

    /// Runs `num_simulations` failing task chains and returns all of them.
    ///
    /// Every trial uses a seed derived from the configured seed, so the trials
    /// explore different random draws while the whole run stays reproducible.
    pub async fn execute_simulation(
        &self,
        num_simulations: usize,
    ) -> Result<RetrySimulation, RetryError> {
        let mut simulation = RetrySimulation::new();
        for index in 0..num_simulations {
            let seed = self.seed.wrapping_add(index as u64);
            simulation.add(self.simulation_with_seed(seed));
        }

        Ok(simulation)
    }

    /// Runs one failing task chain with an explicit seed.
    fn simulation_with_seed(&self, seed: u64) -> SleepSequence {
        let mut state = SimState::new(seed);
        let mut sleeps = Vec::with_capacity(self.max_attempts.saturating_sub(1) as usize);

        for _ in 0..self.max_attempts.saturating_sub(1) {
            sleeps.push(self.backoff.next_sleep(&mut state));
            state.advance();
        }

        SleepSequence::new(sleeps)
    }
}

impl Default for RetrySimulator {
    fn default() -> Self {
        Self::fixed(3, 1000)
    }
}

#[cfg(test)]
mod tests {
    use std::{future::Future, task::Poll};

    use super::{DEFAULT_SEED, RetrySimulator, SimState, SimulatedBackOff, next_interval};
    use crate::backoff::{
        exponential_back_off_policy::ExponentialBackOffPolicy, no_back_off_policy::NoBackOffPolicy,
    };

    /// Drives a future that is expected to complete without ever yielding.
    fn block_on_ready<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("a simulated chain must never wait"),
        }
    }

    #[test]
    fn fixed_backoff_produces_a_deterministic_sequence() {
        let simulator = RetrySimulator::fixed(4, 1000);

        let first = simulator.execute_single_simulation();
        let second = simulator.execute_single_simulation();

        assert_eq!(first.get_sleeps(), &[1000, 1000, 1000]);
        assert_eq!(first.get_sleeps(), second.get_sleeps());
        assert_eq!(first.get_total_sleep(), 3000);
    }

    #[test]
    fn a_single_attempt_never_backs_off() {
        assert!(
            RetrySimulator::fixed(1, 1000)
                .execute_single_simulation()
                .is_empty()
        );
        assert!(
            RetrySimulator::fixed(0, 1000)
                .execute_single_simulation()
                .is_empty()
        );
    }

    #[test]
    fn exponential_backoff_follows_the_policy_configuration() {
        let mut policy = ExponentialBackOffPolicy::default();
        policy.set_initial_interval(1000);
        policy.set_multiplier(2.0);
        policy.set_max_interval(30_000);

        assert_eq!(
            RetrySimulator::exponential(4, policy)
                .execute_single_simulation()
                .get_sleeps(),
            &[1000, 2000, 4000]
        );
    }

    #[test]
    fn exponential_backoff_stops_at_the_maximum_interval() {
        let mut policy = ExponentialBackOffPolicy::default();
        policy.set_initial_interval(1000);
        policy.set_multiplier(2.0);
        policy.set_max_interval(3000);

        assert_eq!(
            RetrySimulator::exponential(5, policy)
                .execute_single_simulation()
                .get_sleeps(),
            &[1000, 2000, 3000, 3000]
        );
    }

    #[test]
    fn exponential_jitter_stays_within_the_configured_bounds() {
        let mut policy = ExponentialBackOffPolicy::with_random();
        policy.set_initial_interval(1000);
        policy.set_multiplier(2.0);
        policy.set_max_interval(3000);
        let simulator = RetrySimulator::exponential(4, policy);

        let sleeps = simulator.execute_single_simulation();
        let sleeps = sleeps.get_sleeps();

        assert_eq!(sleeps.len(), 3);
        assert!((1000..=2000).contains(&sleeps[0]), "was {}", sleeps[0]);
        assert!((2000..=3000).contains(&sleeps[1]), "was {}", sleeps[1]);
        assert_eq!(sleeps[2], 3000);
        assert_eq!(simulator.execute_single_simulation().get_sleeps(), sleeps);
    }

    #[test]
    fn exponential_interval_never_wraps_on_overflow() {
        assert_eq!(next_interval(u64::MAX, 2.0), u64::MAX);
    }

    #[test]
    fn exponential_growth_truncates_the_fractional_part() {
        assert_eq!(next_interval(1001, 1.5), 1501);
    }

    #[test]
    fn uniform_random_backoff_stays_in_range_and_is_reproducible() {
        let simulator = RetrySimulator::uniform_random(5, 500, 1500);

        let first = simulator.execute_single_simulation();
        let second = simulator.execute_single_simulation();

        assert_eq!(first.get_sleeps(), second.get_sleeps());
        assert_eq!(first.len(), 4);
        assert!(
            first
                .get_sleeps()
                .iter()
                .all(|sleep| (500..1500).contains(sleep))
        );
    }

    #[test]
    fn uniform_random_with_inverted_bounds_uses_the_lower_bound() {
        let simulator = RetrySimulator::uniform_random(3, 900, 400);

        assert_eq!(
            simulator.execute_single_simulation().get_sleeps(),
            &[900, 900]
        );
    }

    #[test]
    fn distinct_seeds_change_the_random_draws() {
        let first = RetrySimulator::uniform_random(5, 500, 1500)
            .with_seed(1)
            .execute_single_simulation();
        let second = RetrySimulator::uniform_random(5, 500, 1500)
            .with_seed(2)
            .execute_single_simulation();

        assert_ne!(first.get_sleeps(), second.get_sleeps());
    }

    #[test]
    fn state_never_sticks_to_the_zero_seed() {
        let mut state = SimState::new(0);
        let mut other = SimState::new(DEFAULT_SEED);

        assert_eq!(state.next_u64(), other.next_u64());
        assert_ne!(state.next_u64(), 0);
    }

    #[test]
    fn state_counts_produced_sleeps() {
        let mut state = SimState::default();

        assert_eq!(state.produced(), 0);

        state.advance();

        assert_eq!(state.produced(), 1);
    }

    #[test]
    fn no_backoff_policy_simulates_zero_sleeps() {
        let simulator = RetrySimulator::new(3, NoBackOffPolicy::new());

        assert_eq!(simulator.execute_single_simulation().get_sleeps(), &[0, 0]);
    }

    #[test]
    fn a_custom_backoff_can_be_simulated() {
        struct Doubling {
            sleeps: std::sync::Mutex<Vec<u64>>,
        }

        impl SimulatedBackOff for Doubling {
            fn next_sleep(&self, state: &mut SimState) -> u64 {
                let next = 1u64 << state.produced().min(4);
                if let Ok(mut sleeps) = self.sleeps.lock() {
                    sleeps.push(next);
                }
                next
            }
        }

        let backoff = Doubling {
            sleeps: std::sync::Mutex::new(Vec::new()),
        };
        let simulator = RetrySimulator::new(4, backoff);

        assert_eq!(
            simulator.execute_single_simulation().get_sleeps(),
            &[1, 2, 4]
        );
    }

    #[test]
    fn execute_returns_the_same_sequence_as_the_synchronous_entry_point() {
        let simulator = RetrySimulator::fixed(3, 250);

        let simulation = block_on_ready(simulator.execute()).expect("simulation is infallible");

        assert_eq!(simulation.len(), 1);
        assert_eq!(
            simulation.get_sleep_sequences()[0].get_sleeps(),
            simulator.execute_single_simulation().get_sleeps()
        );
    }

    #[test]
    fn execute_simulation_collects_one_sequence_per_trial() {
        let simulator = RetrySimulator::fixed(4, 1000);

        let simulation =
            block_on_ready(simulator.execute_simulation(3)).expect("simulation is infallible");

        assert_eq!(simulation.len(), 3);
        assert_eq!(simulation.get_sleeps().len(), 9);
        assert_eq!(simulation.get_total_sleep_time(), 9000);
        assert_eq!(simulation.get_longest_sleep(), 1000);
    }

    #[test]
    fn seeds_are_reproducible_across_runs() {
        let simulator = RetrySimulator::uniform_random(4, 100, 200).with_seed(42);

        let first = block_on_ready(simulator.execute_simulation(3))
            .expect("simulation is infallible")
            .get_sleeps()
            .to_vec();
        let second = block_on_ready(simulator.execute_simulation(3))
            .expect("simulation is infallible")
            .get_sleeps()
            .to_vec();

        assert_eq!(first, second);
    }

    #[test]
    fn default_simulator_uses_the_fixed_default_period() {
        let simulator = RetrySimulator::default();

        assert_eq!(simulator.get_max_attempts(), 3);
        assert_eq!(
            simulator.execute_single_simulation().get_sleeps(),
            &[1000, 1000]
        );
    }
}
