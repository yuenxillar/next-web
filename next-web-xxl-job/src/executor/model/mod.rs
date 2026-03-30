use crate::{
    executor::context::job_context::{JobContext, JobHandlerValue},
    web_server::models::admin_req::CallbackParam,
};

#[derive(Clone, Debug)]
pub enum ServerAccessActorReq {
    Stop,
    CallBack(Vec<CallbackParam>),
}

pub enum ServerAccessActorResult {
    None,
}

#[derive(Clone)]
pub enum ExecutorActorReq {
    Register(JobHandlerValue),
    RunJob {
        job_name: String,
        job_context: JobContext,
    },
    IdleBeat {
        job_id: u64,
    },
}

pub enum ExecutorActorResult {
    Ok,
    NotFoundJob,
    Discard,
    JobRunning,
}
