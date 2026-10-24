//! The rolling policy of the log files.
//!
//! A [`RollingPolicy`] describes the files of a log directory: their names, how
//! often a new file is started, and how many of them are kept. A
//! [`RollingFileWriter`] applies a policy while the records are written, so the
//! name of a file is part of the policy rather than of the writer.
//!
//! The names of the files of a directory are:
//!
//! * `<stem>_<date>.<suffix>`, which is `next_20260926.log` for a daily
//!   rotation of a `next.log` file, and `next_20260926_1430.log` for a minutely
//!   rotation;
//! * `<stem>_<date>_<number>.<suffix>`, which is `next_20260926_0.log`, as soon
//!   as the files of a period are split by their size or as soon as the
//!   application starts a new file every time it starts;
//! * `<stem>.<suffix>`, which is `next.log`, when the files are never split by
//!   the time.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{
    DateTime, Datelike as _, FixedOffset, Local, NaiveDate, NaiveDateTime, Timelike as _, Utc,
};
use chrono_tz::Tz;

use crate::context::logging::logging_properties::{LogRotation, LoggingFileProperties};
use crate::context::logging::logging_warnings::WarningReporter;

/// The format of the date of a file name that is dated with a period that holds
/// no part of the date of a day.
const DEFAULT_DATE_PATTERN: &str = "%Y%m%d";

/// The formats of the dates of the file names of the rotations of a day.
const MINUTELY_DATE_PATTERN: &str = "%Y%m%d_%H%M";
/// The format of the date of the file names of an hourly rotation.
const HOURLY_DATE_PATTERN: &str = "%Y%m%d_%H";
/// The format of the date of the file names of a daily and of a weekly
/// rotation.
const DAILY_DATE_PATTERN: &str = "%Y%m%d";

/// The property of the maximum size of a single log file, which is used to
/// report a value that cannot be read.
const MAX_FILE_SIZE_PROPERTY: &str = "next.logging.file.max-file-size";

/// The property of the maximum total size of the log files, which is used to
/// report a value that cannot be read.
const TOTAL_SIZE_CAP_PROPERTY: &str = "next.logging.file.total-size-cap";

/// The property of the time zone of the names of the log files, which is used
/// to report a value that cannot be read.
const TIMEZONE_PROPERTY: &str = "next.logging.file.timezone";

/// The number of bytes of the units a size is written with.
const BYTES_PER_KILOBYTE: u64 = 1024;

/// Returns the size the given value describes, or `None` when the value cannot
/// be read as a size.
///
/// A value is a number that is optionally followed by a unit, for example
/// `10MB` or `512KB`. The units are `B`, `KB`, `MB`, `GB` and `TB`, and they
/// are binary, so `1KB` is 1024 bytes. The units may be written as `KB` or as
/// `KiB`, and a number without a unit is a number of bytes.
///
/// # Arguments
///
/// * `value` - The value to read.
pub fn parse_size(value: &str) -> Option<u64> {
    let value = value.trim();
    let digits = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(digits);

    if number.is_empty() {
        return None;
    }

    let multiplier = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" | "kib" => BYTES_PER_KILOBYTE,
        "m" | "mb" | "mib" => BYTES_PER_KILOBYTE.pow(2),
        "g" | "gb" | "gib" => BYTES_PER_KILOBYTE.pow(3),
        "t" | "tb" | "tib" => BYTES_PER_KILOBYTE.pow(4),
        _ => return None,
    };

    number.parse::<u64>().ok()?.checked_mul(multiplier)
}

/// The time zone the names of the log files are dated in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogTimeZone {
    /// The time zone of the process, which is what the timestamps of the
    /// records use.
    Local,

    /// UTC.
    Utc,

    /// A time zone of the IANA time zone database, such as `Asia/Shanghai`.
    Named(Tz),
}

impl LogTimeZone {
    /// Returns the time zone of the given name, or `None` when the name is not
    /// recognized.
    ///
    /// The name of a time zone of the IANA time zone database is recognized as
    /// well as `local` and `utc`.
    ///
    /// # Arguments
    ///
    /// * `value` - The name of the time zone.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" | "system" | "default" => Some(Self::Local),
            "utc" | "gmt" | "z" => Some(Self::Utc),
            _ => value.trim().parse::<Tz>().ok().map(Self::Named),
        }
    }

    /// Returns the current instant in this time zone.
    pub fn now(&self) -> DateTime<FixedOffset> {
        self.at(Utc::now())
    }

    /// Returns the given instant in this time zone.
    ///
    /// # Arguments
    ///
    /// * `instant` - The instant to convert.
    pub fn at(&self, instant: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self {
            Self::Local => instant.with_timezone(&Local).fixed_offset(),
            Self::Utc => instant.fixed_offset(),
            Self::Named(zone) => instant.with_timezone(zone).fixed_offset(),
        }
    }
}

impl Default for LogTimeZone {
    /// Returns the time zone of the process.
    fn default() -> Self {
        Self::Local
    }
}

/// The policy the log files of a directory are rolled with.
///
/// The policy is created from the file properties of the logging functionality
/// with [`RollingPolicy::resolve`], and the values that cannot be read fall back
/// to the defaults of the properties.
#[derive(Debug, Clone)]
pub struct RollingPolicy {
    directory: PathBuf,
    stem: String,
    suffix: String,
    rotation: LogRotation,
    date_pattern: String,
    time_zone: LogTimeZone,
    max_history: Option<usize>,
    max_file_size: Option<u64>,
    total_size_cap: Option<u64>,
    roll_on_start: bool,
    clean_history_on_start: bool,
}

impl RollingPolicy {
    /// Returns the policy of the given properties.
    ///
    /// # Arguments
    ///
    /// * `file` - The properties of the log file.
    /// * `warnings` - The warnings of the values that cannot be read.
    pub fn resolve(file: &LoggingFileProperties, warnings: &mut Vec<String>) -> Self {
        Self {
            directory: file.directory(),
            stem: file.stem(),
            suffix: file.suffix(),
            rotation: file.rotation(),
            date_pattern: file
                .file_date_format()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| default_date_pattern(file.rotation()).to_owned()),
            time_zone: resolve_time_zone(file.timezone(), warnings),
            max_history: file.max_history(),
            max_file_size: resolve_size(file.max_file_size(), MAX_FILE_SIZE_PROPERTY, warnings),
            total_size_cap: resolve_size(file.total_size_cap(), TOTAL_SIZE_CAP_PROPERTY, warnings),
            roll_on_start: file.roll_on_start(),
            clean_history_on_start: file.clean_history_on_start(),
        }
    }

    /// Returns the directory the log files are stored in.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Returns the stem of the names of the log files.
    pub fn stem(&self) -> &str {
        &self.stem
    }

    /// Returns the extension of the names of the log files.
    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    /// Returns how often a new log file is started.
    pub fn rotation(&self) -> LogRotation {
        self.rotation
    }

    /// Returns the format of the date that is part of the names of the files.
    pub fn date_pattern(&self) -> &str {
        &self.date_pattern
    }

    /// Returns the time zone the names of the files are dated in.
    pub fn time_zone(&self) -> LogTimeZone {
        self.time_zone
    }

    /// Returns the number of the rolled files that are kept in addition to the
    /// file that is written to, or `None` when every file is kept.
    pub fn max_history(&self) -> Option<usize> {
        self.max_history
    }

    /// Returns the maximum size of a single log file, or `None` when the file
    /// is not limited.
    pub fn max_file_size(&self) -> Option<u64> {
        self.max_file_size
    }

    /// Returns the maximum total size of the log files, or `None` when the
    /// directory is not limited.
    pub fn total_size_cap(&self) -> Option<u64> {
        self.total_size_cap
    }

    /// Returns whether a new log file is started when the application starts.
    pub fn roll_on_start(&self) -> bool {
        self.roll_on_start
    }

    /// Returns whether the log files that already exist are removed when the
    /// application starts.
    pub fn clean_history_on_start(&self) -> bool {
        self.clean_history_on_start
    }

    /// Returns whether the names of the files hold the number of the file of
    /// the period.
    ///
    /// The number separates the files of a period that are split by their size,
    /// and the files of the applications that start a new file every time they
    /// start.
    pub fn uses_index(&self) -> bool {
        self.max_file_size.is_some() || self.roll_on_start
    }

    /// Returns the start of the period the given instant belongs to, or `None`
    /// when the files are never split by the time.
    ///
    /// # Arguments
    ///
    /// * `now` - The instant to round.
    pub fn period_of(&self, now: DateTime<FixedOffset>) -> Option<NaiveDateTime> {
        let local = now.naive_local();

        match self.rotation {
            LogRotation::Never => None,
            LogRotation::Minutely => Some(start_of_minute(local)),
            LogRotation::Hourly => Some(start_of_hour(local)),
            LogRotation::Daily => Some(start_of_day(local.date())),
            LogRotation::Weekly => Some(start_of_day(start_of_week(local.date()))),
        }
    }

    /// Returns the name of the file of the given period and number.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the file, which is `None` when the files are
    ///   never split by the time.
    /// * `index` - The number of the file of the period.
    pub fn file_name(&self, period: Option<NaiveDateTime>, index: usize) -> String {
        let mut name = self.file_prefix(period);

        if self.uses_index() {
            name.push('_');
            name.push_str(&index.to_string());
        }

        name.push('.');
        name.push_str(&self.suffix);

        name
    }

    /// Returns the part of the names of the files of the given period that
    /// holds the date of the period.
    ///
    /// The prefix is how the files of a period are found in a directory without
    /// the date of a name having to be read back.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the files, which is `None` when the files are
    ///   never split by the time.
    pub fn file_prefix(&self, period: Option<NaiveDateTime>) -> String {
        let Some(period) = period else {
            return self.stem.clone();
        };

        format!(
            "{}_{}",
            self.stem,
            period.format(&self.date_pattern).to_string()
        )
    }

    /// Returns the path of the file of the given period and number.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the file.
    /// * `index` - The number of the file of the period.
    pub fn file_path(&self, period: Option<NaiveDateTime>, index: usize) -> PathBuf {
        self.directory.join(self.file_name(period, index))
    }

    /// Returns whether the given name is the name of a log file of this policy.
    ///
    /// The name of a file holds the date and the number of the file between the
    /// stem and the extension, so a file that is not a log file of this policy
    /// is not removed by the history of the policy, which is why the part
    /// between the stem and the extension has to be read as a date or as a
    /// number: `next.log` and `next_20260926.log` are log files of the policy
    /// of a `next.log` file, while `next_notes.log` is not.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the file.
    pub fn matches(&self, name: &str) -> bool {
        if name == format!("{}.{}", self.stem, self.suffix) {
            return true;
        }

        let Some(rest) = name.strip_prefix(&format!("{}_", self.stem)) else {
            return false;
        };
        let Some(part) = rest.strip_suffix(&format!(".{}", self.suffix)) else {
            return false;
        };

        is_generated_part(part)
    }

    /// Returns the number the file of the given period has, or `None` when the
    /// name is not the name of a file of the period.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the file.
    /// * `name` - The name of the file.
    pub fn file_index(&self, period: Option<NaiveDateTime>, name: &str) -> Option<usize> {
        let rest = name.strip_prefix(&format!("{}_", self.file_prefix(period)))?;
        let index = rest.strip_suffix(&format!(".{}", self.suffix))?;

        index.parse().ok()
    }

    /// Returns the log files of the directory, the newest first.
    ///
    /// The files are ordered by the time they were written at, so the order of
    /// the files does not depend on the format of the date of their names.
    pub fn history(&self) -> Vec<LogFile> {
        let Ok(entries) = fs::read_dir(&self.directory) else {
            return Vec::new();
        };

        let mut files = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name();
                let name = name.to_str()?;

                if !self.matches(name) {
                    return None;
                }

                let metadata = entry.metadata().ok()?;

                if !metadata.is_file() {
                    return None;
                }

                Some(LogFile {
                    path: entry.path(),
                    size: metadata.len(),
                    modified: metadata.modified().ok(),
                })
            })
            .collect::<Vec<_>>();

        files.sort_by(|left, right| {
            right
                .modified
                .cmp(&left.modified)
                .then_with(|| right.path.cmp(&left.path))
        });

        files
    }

    /// Removes the files that do not fit the number and the size of the history
    /// of this policy.
    ///
    /// The file that is written to is never removed, so the policy bounds the
    /// size of a directory as far as the files of a history can bound it.
    ///
    /// # Arguments
    ///
    /// * `active` - The file that is written to.
    pub fn prune(&self, active: Option<&Path>) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.max_history.is_none() && self.total_size_cap.is_none() {
            return warnings;
        }

        let files = self
            .history()
            .into_iter()
            .filter(|file| Some(file.path()) != active)
            .collect::<Vec<_>>();

        let max_history = self.max_history.unwrap_or(usize::MAX);
        let mut kept = Vec::new();
        let mut total = self.active_size(active);

        for file in files {
            let exceeds_history = kept.len() >= max_history;
            let exceeds_cap = self
                .total_size_cap
                .is_some_and(|cap| total.saturating_add(file.size) > cap);

            if exceeds_history || exceeds_cap {
                if let Err(error) = fs::remove_file(file.path()) {
                    warnings.push(format!(
                        "failed to remove the log file {}: {error}",
                        file.path().display()
                    ));
                }

                continue;
            }

            total = total.saturating_add(file.size);
            kept.push(file);
        }

        warnings
    }

    /// Removes every log file of the directory.
    ///
    /// The files that are not log files of this policy are kept.
    pub fn clean_history(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        for file in self.history() {
            if let Err(error) = fs::remove_file(file.path()) {
                warnings.push(format!(
                    "failed to remove the log file {}: {error}",
                    file.path().display()
                ));
            }
        }

        warnings
    }

    /// Returns the size of the file that is written to, which is 0 when the
    /// file does not exist.
    ///
    /// # Arguments
    ///
    /// * `active` - The file that is written to.
    fn active_size(&self, active: Option<&Path>) -> u64 {
        active
            .and_then(|path| fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    }
}

/// A log file of the directory of a policy.
#[derive(Debug, Clone)]
pub struct LogFile {
    path: PathBuf,
    size: u64,
    modified: Option<SystemTime>,
}

impl LogFile {
    /// Returns the path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the size of the file in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// Writes the records to the log files of a policy and rolls them.
///
/// The writer starts a new file when the period of the policy changes, and when
/// the file that is written to reaches the maximum size of the policy. A record
/// is written to a file as a whole: the writer holds the parts of a record
/// until the record is complete, and only then opens the file it is written to.
pub struct RollingFileWriter {
    policy: RollingPolicy,
    clock: Box<dyn Fn() -> DateTime<FixedOffset> + Send + Sync>,
    file: Option<File>,
    path: Option<PathBuf>,
    period: Option<NaiveDateTime>,
    index: usize,
    bytes: u64,
    buffer: Vec<u8>,
    reporter: WarningReporter,
    last_warning: Option<String>,
}

impl RollingFileWriter {
    /// Creates a writer that dates the names of the files in the time zone of
    /// the policy.
    ///
    /// The file of the period the application starts in is opened, which
    /// creates the directory of the policy when it does not exist.
    ///
    /// # Arguments
    ///
    /// * `policy` - The policy of the files.
    /// * `reporter` - The reporter of the failures of the files.
    ///
    /// # Errors
    ///
    /// Returns the error of the file system when the directory or the file
    /// cannot be created.
    pub fn new(policy: RollingPolicy, reporter: WarningReporter) -> io::Result<Self> {
        let clock = policy.time_zone();
        Self::with_clock(policy, move || clock.now(), reporter)
    }

    /// Creates a writer that reads the current instant from the given clock.
    ///
    /// # Arguments
    ///
    /// * `policy` - The policy of the files.
    /// * `clock` - The clock the writer reads the current instant from.
    /// * `reporter` - The reporter of the failures of the files.
    ///
    /// # Errors
    ///
    /// Returns the error of the file system when the directory or the file
    /// cannot be created.
    pub fn with_clock(
        policy: RollingPolicy,
        clock: impl Fn() -> DateTime<FixedOffset> + Send + Sync + 'static,
        reporter: WarningReporter,
    ) -> io::Result<Self> {
        let mut writer = Self {
            policy,
            clock: Box::new(clock),
            file: None,
            path: None,
            period: None,
            index: 0,
            bytes: 0,
            buffer: Vec::new(),
            reporter,
            last_warning: None,
        };

        writer.open()?;

        Ok(writer)
    }

    /// Returns the policy the files are rolled with.
    pub fn policy(&self) -> &RollingPolicy {
        &self.policy
    }

    /// Returns the path of the file the records are written to, or `None` when
    /// no file is open.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Opens the file of the period the application starts in.
    fn open(&mut self) -> io::Result<()> {
        fs::create_dir_all(&self.policy.directory)?;

        if self.policy.clean_history_on_start() {
            self.report(self.policy.clean_history());
        }

        let period = self.policy.period_of((self.clock)());
        let index = self.index_of(period);

        self.open_file(period, index)?;
        self.prune();

        Ok(())
    }

    /// Returns the number the file of the given period is opened with.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the file.
    fn index_of(&self, period: Option<NaiveDateTime>) -> usize {
        if !self.policy.uses_index() {
            return 0;
        }

        let highest = self
            .policy
            .history()
            .into_iter()
            .filter_map(|file| {
                file.path()
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| self.policy.file_index(period, name))
            })
            .max();

        match (self.policy.roll_on_start(), highest) {
            (true, Some(index)) => index.saturating_add(1),
            (true, None) => 0,
            (false, Some(index)) => index,
            (false, None) => 0,
        }
    }

    /// Opens the file of the given period and number, and adds the records to
    /// it when it already exists.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the file.
    /// * `index` - The number of the file of the period.
    fn open_file(&mut self, period: Option<NaiveDateTime>, index: usize) -> io::Result<()> {
        let path = self.policy.file_path(period, index);
        let file = OpenOptions::new().append(true).create(true).open(&path)?;
        let bytes = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);

        self.file = Some(file);
        self.path = Some(path);
        self.period = period;
        self.index = index;
        self.bytes = bytes;

        Ok(())
    }

    /// Starts the file a new file is written to.
    ///
    /// The file that is open is kept when the new file cannot be created, so
    /// the records are never dropped because of a file that cannot be created.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the new file.
    fn roll(&mut self, period: Option<NaiveDateTime>) -> io::Result<()> {
        if let Some(file) = self.file.as_mut() {
            let _ = file.flush();
        }

        let index = if self.policy.uses_index() && self.period == period {
            self.index.saturating_add(1)
        } else {
            0
        };

        self.open_file(period, index)?;
        self.prune();

        Ok(())
    }

    /// Removes the files that do not fit the history of the policy.
    fn prune(&mut self) {
        self.report(self.policy.prune(self.path.as_deref()));
    }

    /// Reports the given warnings.
    ///
    /// # Arguments
    ///
    /// * `warnings` - The warnings to report.
    fn report(&self, warnings: Vec<String>) {
        for warning in warnings {
            self.reporter.report(warning);
        }
    }

    /// Reports the given warning unless it is the warning that was reported
    /// last, which keeps a failure that repeats itself from filling the channel
    /// of the warnings.
    ///
    /// # Arguments
    ///
    /// * `warning` - The warning to report.
    fn report_once(&mut self, warning: String) {
        if self.last_warning.as_deref() == Some(warning.as_str()) {
            return;
        }

        self.last_warning = Some(warning.clone());
        self.reporter.report(warning);
    }

    /// Returns whether the file a record is written to has to be started before
    /// the record is written.
    ///
    /// # Arguments
    ///
    /// * `period` - The period of the current instant.
    /// * `length` - The length of the record in bytes.
    fn should_roll(&self, period: Option<NaiveDateTime>, length: u64) -> bool {
        if self.period != period {
            return true;
        }

        self.policy
            .max_file_size()
            .is_some_and(|max| self.bytes > 0 && self.bytes.saturating_add(length) > max)
    }

    /// Writes the records the buffer holds to the file they belong to.
    fn emit(&mut self) -> io::Result<()> {
        let period = self.policy.period_of((self.clock)());
        let length = self.buffer.len() as u64;

        if self.should_roll(period, length) {
            if let Err(error) = self.roll(period) {
                // The records are added to the file that is open rather than
                // dropped, and the failure is reported.
                self.report_once(format!("failed to roll the log file: {error}"));
            }
        }

        let Some(file) = self.file.as_mut() else {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "no log file is open",
            ));
        };

        file.write_all(&self.buffer)?;
        self.bytes = self.bytes.saturating_add(length);
        self.buffer.clear();

        Ok(())
    }
}

impl Write for RollingFileWriter {
    /// Adds the given bytes to the buffer of the record that is being written,
    /// and writes the record as a whole as soon as it is complete.
    ///
    /// # Arguments
    ///
    /// * `buf` - The bytes to write.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);

        // The logging functionality ends a record with a newline, so the bytes
        // up to the last newline are the records the file is written with.
        let Some(end) = self.buffer.iter().rposition(|byte| *byte == b'\n') else {
            return Ok(buf.len());
        };

        let rest = self.buffer.split_off(end + 1);
        self.emit()?;
        self.buffer = rest;

        Ok(buf.len())
    }

    /// Writes the bytes of the record that is not complete yet, and flushes the
    /// file.
    fn flush(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            self.emit()?;
        }

        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

impl std::fmt::Debug for RollingFileWriter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RollingFileWriter")
            .field("policy", &self.policy)
            .field("path", &self.path)
            .field("period", &self.period)
            .field("index", &self.index)
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}

/// Returns the default format of the date of the names of the files of a
/// rotation.
///
/// # Arguments
///
/// * `rotation` - The rotation of the files.
fn default_date_pattern(rotation: LogRotation) -> &'static str {
    match rotation {
        LogRotation::Minutely => MINUTELY_DATE_PATTERN,
        LogRotation::Hourly => HOURLY_DATE_PATTERN,
        LogRotation::Daily | LogRotation::Weekly => DAILY_DATE_PATTERN,
        LogRotation::Never => DEFAULT_DATE_PATTERN,
    }
}

/// Returns the time zone of the given name, and adds a warning to the given
/// warnings when the name is not recognized.
///
/// # Arguments
///
/// * `time_zone` - The name of the time zone.
/// * `warnings` - The warnings of the values that cannot be read.
fn resolve_time_zone(time_zone: Option<&str>, warnings: &mut Vec<String>) -> LogTimeZone {
    let Some(time_zone) = time_zone else {
        return LogTimeZone::default();
    };

    match LogTimeZone::parse(time_zone) {
        Some(time_zone) => time_zone,
        None => {
            warnings.push(format!(
                "{TIMEZONE_PROPERTY} '{time_zone}' is not a time zone, using the time zone of the process"
            ));

            LogTimeZone::default()
        }
    }
}

/// Returns the size of the given value, and adds a warning to the given warnings
/// when the value cannot be read as a size.
///
/// # Arguments
///
/// * `size` - The value of the size.
/// * `property` - The property the value was configured with.
/// * `warnings` - The warnings of the values that cannot be read.
fn resolve_size(size: Option<&str>, property: &str, warnings: &mut Vec<String>) -> Option<u64> {
    let size = size?;

    match parse_size(size) {
        Some(size) => Some(size),
        None => {
            warnings.push(format!("{property} '{size}' is not a size"));

            None
        }
    }
}

/// Returns the start of the minute of the given date and time.
///
/// # Arguments
///
/// * `time` - The date and time to round.
fn start_of_minute(time: NaiveDateTime) -> NaiveDateTime {
    time.with_second(0)
        .and_then(|time| time.with_nanosecond(0))
        .unwrap_or(time)
}

/// Returns the start of the hour of the given date and time.
///
/// # Arguments
///
/// * `time` - The date and time to round.
fn start_of_hour(time: NaiveDateTime) -> NaiveDateTime {
    start_of_minute(time)
        .with_minute(0)
        .map(start_of_minute)
        .unwrap_or_else(|| start_of_minute(time))
}

/// Returns the start of the given day.
///
/// # Arguments
///
/// * `date` - The date of the day.
fn start_of_day(date: NaiveDate) -> NaiveDateTime {
    date.and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| date.and_time(chrono::NaiveTime::MIN))
}

/// Returns the date of the Monday of the week of the given date.
///
/// # Arguments
///
/// * `date` - The date of the day.
fn start_of_week(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

/// Returns whether the given part of a name of a log file holds the date and
/// the number of the file.
///
/// The part starts with the year of the date, and it holds the digits of the
/// date together with the characters a date and a number are written with. A
/// part that holds a letter, such as the part of `next_notes.log`, is the part
/// of a name of a file that is not a log file of the policy.
///
/// # Arguments
///
/// * `part` - The part of the name between the stem and the extension.
fn is_generated_part(part: &str) -> bool {
    part.starts_with(|character: char| character.is_ascii_digit())
        && part.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::NaiveTime;
    use next_web_core::env::{BaseEnvironment, ConfigurableEnvironment, MapPropertySource};
    use next_web_core::util::indexmap::IndexMap;

    use crate::context::logging::logging_properties::LoggingProperties;

    use super::*;

    /// Creates an environment holding the given properties.
    fn environment(properties: &[(&str, &str)]) -> BaseEnvironment {
        let mut environment = BaseEnvironment::new();
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect::<IndexMap<_, _>>(),
            )));

        environment
    }

    /// Creates an empty directory the test owns.
    fn directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir()
            .join("next-web-logging-rolling-tests")
            .join(format!("{}-{name}", std::process::id()));

        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    /// Returns the policy of the properties of a log file in the given
    /// directory.
    fn policy(directory: &Path, properties: &[(&str, &str)]) -> RollingPolicy {
        let directory = directory.to_str().unwrap();
        let mut properties = properties.to_vec();
        properties.push(("next.logging.file.path", directory));

        let bound = LoggingProperties::from_environment(&environment(&properties)).unwrap();
        let mut warnings = Vec::new();

        RollingPolicy::resolve(bound.file().unwrap(), &mut warnings)
    }

    /// Returns the names of the files of the given directory.
    fn file_names(directory: &Path) -> Vec<String> {
        let mut names = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();

        names
    }

    /// A clock the tests move themselves.
    struct TestClock(Arc<Mutex<DateTime<FixedOffset>>>);

    impl TestClock {
        /// Creates a clock that reads the given instant, which is written in
        /// UTC.
        fn new(instant: &str) -> Self {
            let instant = DateTime::parse_from_rfc3339(instant).unwrap();

            Self(Arc::new(Mutex::new(instant)))
        }

        /// Moves the clock to the given instant, which is written in UTC.
        fn set(&self, instant: &str) {
            *self.0.lock().unwrap() = DateTime::parse_from_rfc3339(instant).unwrap();
        }

        /// Returns a function that reads the instant of the clock.
        fn reader(&self) -> impl Fn() -> DateTime<FixedOffset> + Send + Sync + 'static {
            let instant = Arc::clone(&self.0);

            move || *instant.lock().unwrap()
        }
    }

    /// Creates a writer of the given policy that writes the records at the time
    /// of the given clock.
    fn writer(policy: RollingPolicy, clock: &TestClock) -> RollingFileWriter {
        RollingFileWriter::with_clock(policy, clock.reader(), WarningReporter::none()).unwrap()
    }

    /// Writes the given records to the writer.
    fn write(writer: &mut RollingFileWriter, records: &[&str]) {
        for record in records {
            writer.write_all(format!("{record}\n").as_bytes()).unwrap();
        }

        writer.flush().unwrap();
    }

    /// Returns the contents of the given file.
    fn contents(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn reads_the_sizes_of_the_units() {
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("10B"), Some(10));
        assert_eq!(parse_size("10KB"), Some(10 * 1024));
        assert_eq!(parse_size("10kb"), Some(10 * 1024));
        assert_eq!(parse_size("10 KiB"), Some(10 * 1024));
        assert_eq!(parse_size("10MB"), Some(10 * 1024 * 1024));
        assert_eq!(parse_size("1GB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("1TB"), Some(1024 * 1024 * 1024 * 1024));
        assert_eq!(parse_size("10XB"), None);
        assert_eq!(parse_size("MB"), None);
        assert_eq!(parse_size(""), None);
    }

    #[test]
    fn reads_the_time_zones() {
        assert_eq!(LogTimeZone::parse("local"), Some(LogTimeZone::Local));
        assert_eq!(LogTimeZone::parse("UTC"), Some(LogTimeZone::Utc));
        assert_eq!(
            LogTimeZone::parse("Asia/Shanghai"),
            Some(LogTimeZone::Named(chrono_tz::Asia::Shanghai))
        );
        assert_eq!(LogTimeZone::parse("Tomorrow/Land"), None);
    }

    #[test]
    fn converts_an_instant_to_a_time_zone() {
        let instant = DateTime::parse_from_rfc3339("2026-09-26T20:30:00Z")
            .unwrap()
            .to_utc();
        let shanghai = LogTimeZone::Named(chrono_tz::Asia::Shanghai);

        assert_eq!(
            LogTimeZone::Utc.at(instant).naive_local(),
            instant.naive_utc()
        );
        assert_eq!(
            shanghai.at(instant).naive_local(),
            NaiveDate::from_ymd_opt(2026, 9, 27)
                .unwrap()
                .and_hms_opt(4, 30, 0)
                .unwrap()
        );
    }

    #[test]
    fn names_the_files_of_every_rotation() {
        let directory = directory("names-the-files");
        let period = NaiveDate::from_ymd_opt(2026, 9, 26)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap();

        for (rotation, name) in [
            ("never", "next.log"),
            ("minutely", "next_20260926_1430.log"),
            ("hourly", "next_20260926_14.log"),
            ("daily", "next_20260926.log"),
            ("weekly", "next_20260926.log"),
        ] {
            let policy = policy(&directory, &[("next.logging.file.rotation", rotation)]);
            let period = match policy.rotation() {
                LogRotation::Never => None,
                _ => Some(period),
            };

            assert_eq!(policy.file_name(period, 0), name, "rotation: {rotation}");
        }
    }

    #[test]
    fn names_the_files_with_a_number_when_the_files_are_split() {
        let directory = directory("names-the-files-with-a-number");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.max-file-size", "10MB"),
            ],
        );
        let period = NaiveDate::from_ymd_opt(2026, 9, 26)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        assert!(policy.uses_index());
        assert_eq!(policy.file_name(Some(period), 0), "next_20260926_0.log");
        assert_eq!(policy.file_name(Some(period), 1), "next_20260926_1.log");
    }

    #[test]
    fn names_the_files_with_the_format_of_the_property() {
        let directory = directory("names-the-files-with-a-format");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.date-format", "%Y_%m_%d"),
            ],
        );
        let period = NaiveDate::from_ymd_opt(2026, 9, 26)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        assert_eq!(policy.file_name(Some(period), 0), "next_2026_09_26.log");
        assert_eq!(policy.file_prefix(Some(period)), "next_2026_09_26");
    }

    #[test]
    fn reads_the_period_of_an_instant() {
        let directory = directory("reads-the-period");
        let instant = DateTime::parse_from_rfc3339("2026-09-26T14:30:45Z")
            .unwrap()
            .to_utc();
        let instant = LogTimeZone::Utc.at(instant);
        let date = |year, month, day| {
            NaiveDate::from_ymd_opt(year, month, day)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        };

        let period = |rotation| {
            policy(&directory, &[("next.logging.file.rotation", rotation)]).period_of(instant)
        };

        assert_eq!(period("never"), None);
        assert_eq!(
            period("minutely"),
            Some(
                NaiveDate::from_ymd_opt(2026, 9, 26)
                    .unwrap()
                    .and_time(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
            )
        );
        assert_eq!(
            period("hourly"),
            Some(
                NaiveDate::from_ymd_opt(2026, 9, 26)
                    .unwrap()
                    .and_hms_opt(14, 0, 0)
                    .unwrap()
            )
        );
        assert_eq!(period("daily"), Some(date(2026, 9, 26)));
        // The 2026-09-26 is a Saturday, so its week starts on the 2026-09-21.
        assert_eq!(period("weekly"), Some(date(2026, 9, 21)));
    }

    #[test]
    fn reads_the_number_of_the_file_of_a_period() {
        let directory = directory("reads-the-number-of-a-file");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.max-file-size", "10MB"),
            ],
        );
        let period = NaiveDate::from_ymd_opt(2026, 9, 26)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        assert_eq!(policy.file_prefix(Some(period)), "next_20260926");
        assert_eq!(
            policy.file_index(Some(period), "next_20260926_2.log"),
            Some(2)
        );
        assert_eq!(policy.file_index(Some(period), "next_20260925_2.log"), None);
        assert_eq!(policy.file_index(Some(period), "next.log"), None);
        assert_eq!(policy.file_index(Some(period), "application.log"), None);
    }

    #[test]
    fn does_not_match_the_files_that_are_not_log_files_of_the_policy() {
        let directory = directory("does-not-match-the-files");
        let policy = policy(&directory, &[("next.logging.file.rotation", "daily")]);

        assert!(policy.matches("next.log"));
        assert!(policy.matches("next_20260926.log"));
        assert!(policy.matches("next_20260926_1.log"));
        assert!(!policy.matches("next_notes.log"));
        assert!(!policy.matches("next_20260926.txt"));
        assert!(!policy.matches("other_20260926.log"));
    }

    #[test]
    fn writes_the_records_to_the_file_of_the_period() {
        let directory = directory("writes-the-records");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        let policy = policy(&directory, &[("next.logging.file.rotation", "daily")]);
        let mut writer = writer(policy, &clock);

        write(&mut writer, &["first", "second"]);

        assert_eq!(file_names(&directory), vec!["next_20260926.log"]);
        assert_eq!(
            contents(&directory.join("next_20260926.log")),
            "first\nsecond\n"
        );
    }

    #[test]
    fn starts_a_new_file_when_the_period_changes() {
        let directory = directory("starts-a-new-file");
        let clock = TestClock::new("2026-09-26T23:59:00Z");
        let policy = policy(&directory, &[("next.logging.file.rotation", "daily")]);
        let mut writer = writer(policy, &clock);

        write(&mut writer, &["before"]);
        clock.set("2026-09-27T00:01:00Z");
        write(&mut writer, &["after"]);

        assert_eq!(
            file_names(&directory),
            vec!["next_20260926.log", "next_20260927.log"]
        );
        assert_eq!(contents(&directory.join("next_20260926.log")), "before\n");
        assert_eq!(contents(&directory.join("next_20260927.log")), "after\n");
    }

    #[test]
    fn starts_a_new_file_when_the_size_is_reached() {
        let directory = directory("starts-a-new-file-by-size");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        // Every record is 7 bytes long, so a file of 14 bytes holds 2 of them.
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.max-file-size", "14B"),
            ],
        );
        let mut writer = writer(policy, &clock);

        write(&mut writer, &["record", "record", "record"]);

        assert_eq!(
            file_names(&directory),
            vec!["next_20260926_0.log", "next_20260926_1.log"]
        );
        assert_eq!(
            contents(&directory.join("next_20260926_0.log")),
            "record\nrecord\n"
        );
        assert_eq!(contents(&directory.join("next_20260926_1.log")), "record\n");
    }

    #[test]
    fn adds_the_records_to_the_file_of_the_period_of_a_new_application() {
        let directory = directory("adds-the-records-of-a-new-application");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        let policy = policy(&directory, &[("next.logging.file.rotation", "daily")]);
        std::fs::write(directory.join("next_20260926.log"), "before\n").unwrap();

        let mut writer = writer(policy, &clock);
        write(&mut writer, &["after"]);

        assert_eq!(file_names(&directory), vec!["next_20260926.log"]);
        assert_eq!(
            contents(&directory.join("next_20260926.log")),
            "before\nafter\n"
        );
    }

    #[test]
    fn splits_the_file_of_a_policy_that_is_not_split_by_the_time() {
        let directory = directory("splits-the-file-of-a-policy");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        // Every record is 7 bytes long, so a file of 20 bytes holds 2 of them.
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "never"),
                ("next.logging.file.max-file-size", "20B"),
            ],
        );
        let mut writer = writer(policy, &clock);

        write(&mut writer, &["record", "record", "record", "record"]);

        assert_eq!(file_names(&directory), vec!["next_0.log", "next_1.log"]);
        assert_eq!(contents(&directory.join("next_0.log")), "record\nrecord\n");
        assert_eq!(contents(&directory.join("next_1.log")), "record\nrecord\n");
    }

    #[test]
    fn starts_a_new_file_when_the_application_starts() {
        let directory = directory("starts-a-new-file-of-an-application");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.roll-on-start", "true"),
            ],
        );
        std::fs::write(directory.join("next_20260926_0.log"), "before\n").unwrap();

        let mut writer = writer(policy, &clock);
        write(&mut writer, &["after"]);

        assert_eq!(
            file_names(&directory),
            vec!["next_20260926_0.log", "next_20260926_1.log"]
        );
        assert_eq!(contents(&directory.join("next_20260926_1.log")), "after\n");
    }

    #[test]
    fn keeps_the_number_of_the_files_of_the_history() {
        let directory = directory("keeps-the-history");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.max-history", "2"),
            ],
        );

        for day in 1..=4 {
            std::fs::write(
                directory.join(format!("next_2026090{day}.log")),
                "a record\n",
            )
            .unwrap();
        }

        let mut writer = writer(policy, &clock);
        write(&mut writer, &["today"]);

        assert_eq!(
            file_names(&directory),
            vec![
                "next_20260903.log",
                "next_20260904.log",
                "next_20260926.log"
            ]
        );
    }

    #[test]
    fn keeps_the_total_size_of_the_history_below_the_cap() {
        let directory = directory("keeps-the-size");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.total-size-cap", "25B"),
            ],
        );

        for day in 1..=4 {
            std::fs::write(
                directory.join(format!("next_2026090{day}.log")),
                "0123456789",
            )
            .unwrap();
        }

        let mut writer = writer(policy, &clock);
        write(&mut writer, &["today"]);

        // The files of the 3rd and of the 4th are 20 bytes together, so the
        // files of the 1st and of the 2nd are removed.
        assert_eq!(
            file_names(&directory),
            vec![
                "next_20260903.log",
                "next_20260904.log",
                "next_20260926.log"
            ]
        );
    }

    #[test]
    fn keeps_the_file_that_is_written_to() {
        let directory = directory("keeps-the-file-that-is-written-to");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.total-size-cap", "1B"),
                ("next.logging.file.max-history", "0"),
            ],
        );
        let mut writer = writer(policy, &clock);

        write(&mut writer, &["a record that does not fit the cap"]);

        let file = writer.path().unwrap().to_owned();

        assert!(file.exists(), "the file that is written to was removed");
        assert_eq!(contents(&file), "a record that does not fit the cap\n");
    }

    #[test]
    fn removes_the_history_of_a_new_application() {
        let directory = directory("removes-the-history");
        let clock = TestClock::new("2026-09-26T14:30:00Z");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.clean-history-on-start", "true"),
            ],
        );
        std::fs::write(directory.join("next_20260901.log"), "before\n").unwrap();
        std::fs::write(directory.join("next.log"), "before\n").unwrap();
        std::fs::write(directory.join("keep.txt"), "keep\n").unwrap();

        let mut writer = writer(policy, &clock);
        write(&mut writer, &["after"]);

        assert_eq!(
            file_names(&directory),
            vec!["keep.txt", "next_20260926.log"]
        );
        assert_eq!(contents(&directory.join("next_20260926.log")), "after\n");
    }

    #[test]
    fn reports_the_values_that_cannot_be_read() {
        let directory = directory("reports-the-values");
        let bound = LoggingProperties::from_environment(&environment(&[
            ("next.logging.file.path", directory.to_str().unwrap()),
            ("next.logging.file.max-file-size", "10XB"),
            ("next.logging.file.total-size-cap", "big"),
            ("next.logging.file.timezone", "Tomorrow/Land"),
        ]))
        .unwrap();

        let mut warnings = Vec::new();
        let policy = RollingPolicy::resolve(bound.file().unwrap(), &mut warnings);

        assert_eq!(policy.max_file_size(), None);
        assert_eq!(policy.total_size_cap(), None);
        assert_eq!(policy.time_zone(), LogTimeZone::Local);
        assert_eq!(warnings.len(), 3);
    }

    #[test]
    fn names_the_file_of_the_period_of_the_time_zone_of_the_policy() {
        let directory = directory("names-the-file-in-a-time-zone");
        let policy = policy(
            &directory,
            &[
                ("next.logging.file.rotation", "daily"),
                ("next.logging.file.timezone", "Asia/Shanghai"),
            ],
        );
        let instant = DateTime::parse_from_rfc3339("2026-09-26T20:30:00Z")
            .unwrap()
            .to_utc();
        let time_zone = policy.time_zone();
        // The instant is the 2026-09-27 04:30 in Shanghai, so the file of the
        // 27th is written rather than the file of the 26th.
        let mut writer = RollingFileWriter::with_clock(
            policy,
            move || time_zone.at(instant),
            WarningReporter::none(),
        )
        .unwrap();

        write(&mut writer, &["a record"]);

        assert_eq!(file_names(&directory), vec!["next_20260927.log"]);
    }

    #[test]
    fn reports_the_file_that_cannot_be_created() {
        let directory = directory("reports-the-file-of-a-roll");
        let clock = TestClock::new("2026-09-26T23:59:00Z");
        let (reporter, warnings) = WarningReporter::channel();
        let policy = policy(&directory, &[("next.logging.file.rotation", "daily")]);

        // The directory of the name of the file of the next day keeps the file
        // from being created.
        std::fs::create_dir_all(directory.join("next_20260927.log")).unwrap();

        let mut writer = RollingFileWriter::with_clock(policy, clock.reader(), reporter).unwrap();

        write(&mut writer, &["before"]);
        clock.set("2026-09-27T00:01:00Z");
        write(&mut writer, &["after"]);

        let warning = warnings
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("the failure of the roll was not reported");

        assert!(
            warning.starts_with("failed to roll the log file"),
            "unexpected warning: {warning}"
        );
        // The records are added to the file that is open rather than dropped.
        assert_eq!(
            contents(&directory.join("next_20260926.log")),
            "before\nafter\n"
        );
        assert_eq!(
            writer.path(),
            Some(directory.join("next_20260926.log").as_path())
        );
    }
}
