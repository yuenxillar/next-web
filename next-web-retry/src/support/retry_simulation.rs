//! Results of a retry simulation.
//!
//! A simulation collects the sleep durations that a backoff policy would have
//! requested for a series of failing task chains. The collected data answers
//! questions such as "what is the typical sleep of a retry sequence?", "how
//! long does the worst sequence sleep in total?" and "is the configured maximum
//! interval ever reached?".
//!
//! Every sleep is expressed in milliseconds and stored as `u64`; no time is
//! actually spent sleeping while simulating.

use std::fmt;

/// The sleeps produced by a single simulated retry sequence.
///
/// A sequence holds the sleeps in the order they were produced, together with
/// the two aggregates that callers ask about most often: the longest single
/// sleep and the total sleep time.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SleepSequence {
    sleeps: Vec<u64>,
    longest_sleep: u64,
    total_sleep: u64,
}

impl SleepSequence {
    /// Creates a sequence from the sleeps, in the order they were produced.
    ///
    /// The aggregates are computed eagerly and saturating, so a long sequence
    /// can never overflow the total.
    pub fn new(sleeps: Vec<u64>) -> Self {
        let longest_sleep = sleeps.iter().copied().max().unwrap_or_default();
        let total_sleep = sleeps.iter().copied().fold(0, u64::saturating_add);

        Self {
            sleeps,
            longest_sleep,
            total_sleep,
        }
    }

    /// Returns the sleeps in the order they were produced.
    pub fn get_sleeps(&self) -> &[u64] {
        &self.sleeps
    }

    /// Returns the longest single sleep of the sequence.
    ///
    /// An empty sequence has no longest sleep and reports `0`.
    pub fn get_longest_sleep(&self) -> u64 {
        self.longest_sleep
    }

    /// Returns the sum of every sleep of the sequence.
    pub fn get_total_sleep(&self) -> u64 {
        self.total_sleep
    }

    /// Returns the number of sleeps of the sequence.
    pub fn len(&self) -> usize {
        self.sleeps.len()
    }

    /// Returns `true` when the sequence contains no sleep at all.
    ///
    /// This happens when the retry chain only has a single attempt, so no
    /// backoff is applied.
    pub fn is_empty(&self) -> bool {
        self.sleeps.is_empty()
    }
}

impl From<Vec<u64>> for SleepSequence {
    fn from(sleeps: Vec<u64>) -> Self {
        Self::new(sleeps)
    }
}

impl fmt::Display for SleepSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "total_sleep={}: {:?}", self.total_sleep, self.sleeps)
    }
}

/// The aggregated outcome of one or more simulated retry sequences.
///
/// The simulation keeps the individual sequences, so the worst sequence can be
/// inspected, and a flat histogram of every individual sleep, so percentiles
/// can be computed over the whole population.
#[derive(Clone, Debug, Default)]
pub struct RetrySimulation {
    sleep_sequences: Vec<SleepSequence>,
    /// Every individual sleep of every sequence, kept sorted ascending.
    sleep_histogram: Vec<u64>,
}

impl RetrySimulation {
    /// Creates an empty simulation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an already built sequence to the simulation.
    pub fn add(&mut self, sequence: SleepSequence) {
        for sleep in sequence.get_sleeps() {
            // Keep the histogram sorted on insertion so percentile queries do
            // not have to sort on every call.
            let index = self.sleep_histogram.partition_point(|value| value <= sleep);
            self.sleep_histogram.insert(index, *sleep);
        }

        self.sleep_sequences.push(sequence);
    }

    /// Adds a sequence of sleeps to the simulation.
    pub fn add_sequence(&mut self, sleeps: Vec<u64>) {
        self.add(SleepSequence::new(sleeps));
    }

    /// Returns every individual sleep of every sequence, sorted ascending.
    pub fn get_sleeps(&self) -> &[u64] {
        &self.sleep_histogram
    }

    /// Returns every recorded sequence, in insertion order.
    pub fn get_sleep_sequences(&self) -> &[SleepSequence] {
        &self.sleep_sequences
    }

    /// Returns the number of recorded sequences.
    pub fn len(&self) -> usize {
        self.sleep_sequences.len()
    }

    /// Returns `true` when no sequence has been recorded.
    pub fn is_empty(&self) -> bool {
        self.sleep_sequences.is_empty()
    }

    /// Returns the total time slept across every recorded sequence.
    pub fn get_total_sleep_time(&self) -> u64 {
        self.sleep_histogram
            .iter()
            .copied()
            .fold(0, u64::saturating_add)
    }

    /// Returns the longest individual sleep across every recorded sequence.
    pub fn get_longest_sleep(&self) -> u64 {
        self.sleep_histogram.last().copied().unwrap_or_default()
    }

    /// Returns the percentiles `10, 20, ... 90` of the recorded sleeps.
    ///
    /// Each entry is [`Self::get_percentile`] evaluated at `0.1 ..= 0.9`.
    pub fn get_percentiles(&self) -> Vec<f64> {
        [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0]
            .into_iter()
            .map(|percentile| self.get_percentile(percentile / 100.0))
            .collect()
    }

    /// Returns the interpolated percentile `p` of the recorded sleeps.
    ///
    /// The value is selected from the ordered sleeps: the fractional position
    /// `p * (n - 1)` is computed, and the result linearly interpolates between
    /// the sleep at `floor(position)` and the one after it, weighted by the
    /// fractional part.
    ///
    /// Degenerate inputs are answered instead of failing:
    ///
    /// * an empty simulation returns `0.0`;
    /// * a single recorded sleep returns that sleep for every `p`;
    /// * `p` is clamped to `0.0 ..= 1.0`, and a non-finite `p` is treated as
    ///   `0.0`.
    pub fn get_percentile(&self, p: f64) -> f64 {
        let size = self.sleep_histogram.len();
        if size == 0 {
            return 0.0;
        }

        let p = if p.is_nan() { 0.0 } else { p.clamp(0.0, 1.0) };
        let position = p * (size - 1) as f64;
        let lower_index = position.floor() as usize;
        // The upper neighbour does not exist for a single-element histogram.
        let upper_index = (lower_index + 1).min(size - 1);
        let weight = position - lower_index as f64;

        let lower = self.sleep_histogram[lower_index] as f64;
        let upper = self.sleep_histogram[upper_index] as f64;

        lower * (1.0 - weight) + upper * weight
    }

    /// Returns the sequence with the longest total sleep.
    ///
    /// When several sequences share the longest total, the first one inserted
    /// wins. An empty simulation has no such sequence.
    pub fn get_longest_total_sleep_sequence(&self) -> Option<&SleepSequence> {
        let mut longest: Option<&SleepSequence> = None;
        for sequence in &self.sleep_sequences {
            let is_longer = longest
                .is_none_or(|current| sequence.get_total_sleep() > current.get_total_sleep());
            if is_longer {
                longest = Some(sequence);
            }
        }

        longest
    }
}

#[cfg(test)]
mod tests {
    use super::{RetrySimulation, SleepSequence};

    /// Compares two floating point values with a tolerance that hides the
    /// rounding noise of the interpolation.
    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn sequence_reports_longest_and_total_sleep() {
        let sequence = SleepSequence::new(vec![300, 100, 200]);

        assert_eq!(sequence.get_sleeps(), &[300, 100, 200]);
        assert_eq!(sequence.get_longest_sleep(), 300);
        assert_eq!(sequence.get_total_sleep(), 600);
        assert_eq!(sequence.len(), 3);
        assert!(!sequence.is_empty());
    }

    #[test]
    fn empty_sequence_aggregates_to_zero() {
        let sequence = SleepSequence::default();

        assert!(sequence.is_empty());
        assert_eq!(sequence.get_longest_sleep(), 0);
        assert_eq!(sequence.get_total_sleep(), 0);
        assert_eq!(sequence.to_string(), "total_sleep=0: []");
    }

    #[test]
    fn simulation_accumulates_sequences_and_totals() {
        let mut simulation = RetrySimulation::new();
        simulation.add_sequence(vec![1000, 2000]);
        simulation.add(SleepSequence::new(vec![500]));

        assert_eq!(simulation.len(), 2);
        assert!(!simulation.is_empty());
        assert_eq!(simulation.get_sleeps(), &[500, 1000, 2000]);
        assert_eq!(simulation.get_total_sleep_time(), 3500);
        assert_eq!(simulation.get_longest_sleep(), 2000);
    }

    #[test]
    fn empty_simulation_has_no_sequences_or_sleeps() {
        let simulation = RetrySimulation::default();

        assert!(simulation.is_empty());
        assert!(simulation.get_sleeps().is_empty());
        assert_eq!(simulation.get_total_sleep_time(), 0);
        assert_eq!(simulation.get_longest_sleep(), 0);
        assert!(simulation.get_longest_total_sleep_sequence().is_none());
    }

    #[test]
    fn percentile_interpolates_between_neighbours() {
        let mut simulation = RetrySimulation::new();
        simulation.add_sequence(vec![400, 100, 300, 200]);

        assert_close(simulation.get_percentile(0.0), 100.0);
        assert_close(simulation.get_percentile(0.25), 175.0);
        assert_close(simulation.get_percentile(0.5), 250.0);
        assert_close(simulation.get_percentile(0.75), 325.0);
        assert_close(simulation.get_percentile(1.0), 400.0);
    }

    #[test]
    fn percentiles_cover_the_deciles() {
        let mut simulation = RetrySimulation::new();
        simulation.add_sequence(vec![100, 200, 300, 400]);

        let percentiles = simulation.get_percentiles();

        assert_eq!(percentiles.len(), 9);
        assert_close(percentiles[0], 130.0);
        assert_close(percentiles[4], 250.0);
        assert_close(percentiles[8], 370.0);
    }

    #[test]
    fn percentile_of_single_sleep_is_that_sleep() {
        let mut simulation = RetrySimulation::new();
        simulation.add_sequence(vec![700]);

        assert_close(simulation.get_percentile(0.0), 700.0);
        assert_close(simulation.get_percentile(0.5), 700.0);
        assert_close(simulation.get_percentile(0.9), 700.0);
    }

    #[test]
    fn percentile_of_empty_simulation_is_zero() {
        let simulation = RetrySimulation::default();

        assert_close(simulation.get_percentile(0.5), 0.0);
        assert_close(simulation.get_percentile(f64::NAN), 0.0);
    }

    #[test]
    fn percentile_clamps_out_of_range_probabilities() {
        let mut simulation = RetrySimulation::new();
        simulation.add_sequence(vec![100, 200]);

        assert_close(simulation.get_percentile(-1.0), 100.0);
        assert_close(simulation.get_percentile(2.0), 200.0);
    }

    #[test]
    fn longest_total_sleep_sequence_keeps_the_first_of_a_tie() {
        let mut simulation = RetrySimulation::new();
        simulation.add_sequence(vec![100, 200]);
        simulation.add_sequence(vec![300]);
        simulation.add_sequence(vec![50, 50]);

        let longest = simulation.get_longest_total_sleep_sequence();

        assert_eq!(longest.map(SleepSequence::get_total_sleep), Some(300));
        assert_eq!(
            longest.map(|sequence| sequence.get_sleeps().to_vec()),
            Some(vec![100, 200])
        );
    }
}
