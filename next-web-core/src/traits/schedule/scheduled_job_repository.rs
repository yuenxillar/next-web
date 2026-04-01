use crate::traits::schedule::{
    scheduled_job_reader::ScheduledJobReader, scheduled_job_store::ScheduledJobStore,
};

pub trait ScheduledJobRepository
where
    Self: ScheduledJobReader,
    Self: ScheduledJobStore,
{
}

impl<T> ScheduledJobRepository for T where T: ScheduledJobReader + ScheduledJobStore {}
