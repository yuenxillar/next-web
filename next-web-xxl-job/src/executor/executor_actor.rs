use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use axum::BoxError;
use tokio::sync::Mutex;

use crate::executor::admin_server::{ServerAccessActor, callback};
use crate::executor::context::job_context::{
    JobContext, JobHandler, JobHandlerRunParam, JobHandlerValue,
};
use crate::executor::models::{ExecutorActorReq, ExecutorActorResult};
use crate::web_server::config::client_config::ClientConfig;
use crate::web_server::models::api_result::FAIL_CODE;
use crate::web_server::models::enum_type::ExecutorBlockStrategy;

#[derive(Clone, Default)]
pub struct ExecutorActor {
    #[allow(unused)]
    client_config: Arc<ClientConfig>,
    job_handler_map: Arc<Mutex<HashMap<Arc<String>, JobHandlerValue>>>,
    job_id_map: Arc<Mutex<HashMap<u64, Arc<String>>>>,
    server_access_actor: Option<ServerAccessActor>,
}

impl ExecutorActor {
    pub fn new(client_config: Arc<ClientConfig>) -> Self {
        Self {
            client_config,
            job_handler_map: Arc::new(Mutex::new(HashMap::new())),
            job_id_map: Arc::new(Mutex::new(HashMap::new())),
            server_access_actor: None,
        }
    }

    pub async fn send(&self, msg: ExecutorActorReq) -> Result<ExecutorActorResult, BoxError> {
        match msg {
            ExecutorActorReq::Register(job_handler_value) => {
                self.register_job_handler(job_handler_value).await;
                Ok(ExecutorActorResult::Ok)
            }
            ExecutorActorReq::RunJob {
                job_name,
                job_context,
            } => self.run_job(Arc::new(job_name), job_context).await,
            ExecutorActorReq::IdleBeat { job_id } => self.check_idle_beat(job_id).await,
        }
    }

    async fn register_job_handler(&self, job_handler: JobHandlerValue) {
        self.job_handler_map
            .lock()
            .await
            .insert(job_handler.name.clone(), job_handler);
    }

    async fn run_job(
        &self,
        job_name: Arc<String>,
        job_context: JobContext,
    ) -> Result<ExecutorActorResult, BoxError> {
        self.job_id_map
            .lock()
            .await
            .entry(job_context.job_id)
            .or_insert_with(|| job_name.clone());
        let run_param =
            if let Some(handler_value) = self.job_handler_map.lock().await.get_mut(&job_name) {
                if handler_value.is_running {
                    match &job_context.block_strategy {
                        ExecutorBlockStrategy::SerialExecution => {
                            //如果超过排队上限会移除前面任务
                            if let Some(old_job) = handler_value.push_block_job(job_context) {
                                old_job.callback_failed().await;
                            }
                            return Ok(ExecutorActorResult::Ok);
                        }
                        ExecutorBlockStrategy::DiscardLater => {
                            job_context
                                .callback_failed_with_info(
                                    format!(
                                        "Discard the job; job_id:{}, log_id:{}",
                                        job_context.job_id, job_context.log_id
                                    ),
                                    FAIL_CODE,
                                )
                                .await;
                            return Ok(ExecutorActorResult::Discard);
                        }
                        ExecutorBlockStrategy::CoverEarly | ExecutorBlockStrategy::Other => {}
                    }
                }
                handler_value.is_running = true;
                handler_value.last_run_id = job_context.log_id;
                handler_value.build_run_param()
            } else {
                return Err(
                    format!("No handler registered for job: {}", job_name.as_str())
                        .as_str()
                        .into(),
                );
            };

        let actor = self.clone();
        tokio::spawn(Self::do_run_job(actor, job_context, run_param));

        Ok(ExecutorActorResult::Ok)
    }

    fn do_run_job(
        actor: Self,
        job_context: JobContext,
        job_handler_param: JobHandlerRunParam,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            let job_handler = job_handler_param.handler.clone();
            let job_name = job_handler_param.name.clone();
            let log_id = job_context.log_id.to_owned();

            let job_handler = job_handler.clone();
            // 根据任务类型分别处理
            let (result, job_name, log_id) = match job_handler {
                // 异步任务：直接 await
                JobHandler::Async(handler) => {
                    (handler.process(job_context).await, job_name, log_id)
                }

                // 同步任务：在独立线程中运行，避免阻塞异步运行时
                JobHandler::Sync(handler) => {
                    let (tx, rx) = tokio::sync::oneshot::channel();

                    std::thread::spawn(move || {
                        let ctx = handler.process(job_context);
                        tx.send(ctx).ok();
                    });

                    let res = match rx.await {
                        Ok(v) => v,
                        Err(e) => Err(e.into()),
                    };
                    (res, job_name, log_id)
                }
            };

            match result {
                Ok(job) => {
                    job.callback_success().await;
                }
                Err(err) => {
                    // 失败回调
                    if let Some(addr) = actor.server_access_actor.as_ref() {
                        callback(addr, log_id.to_owned(), FAIL_CODE, Some(err.to_string())).await;
                    }
                }
            };

            let mut is_empty = false;
            // 更新任务状态并触发下一个任务
            if let Some(value) = actor.job_handler_map.lock().await.get_mut(&job_name) {
                if value.last_run_id == log_id {
                    value.is_running = false; // 标记任务结束
                    value.last_run_id = 0;
                }

                is_empty = value.block_jobs.is_empty();
            };

            if !is_empty {
                Self::run_next_job(actor, job_name).await;
            }
        })
    }

    fn run_next_job(
        actor: Self,
        job_name: Arc<String>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            let (job_context, run_param) =
                if let Some(value) = actor.job_handler_map.lock().await.get_mut(&job_name) {
                    if let Some(job) = value.pop_block_job() {
                        value.is_running = true;
                        value.last_run_id = job.log_id;
                        (job, value.build_run_param())
                    } else {
                        return;
                    }
                } else {
                    return;
                };

            Self::do_run_job(actor, job_context, run_param).await;
        })
    }

    async fn check_idle_beat(&self, job_id: u64) -> Result<ExecutorActorResult, BoxError> {
        if let Some(name) = self.job_id_map.lock().await.get(&job_id) {
            if let Some(handler) = self.job_handler_map.lock().await.get_mut(name) {
                if handler.is_running || !handler.block_jobs.is_empty() {
                    return Ok(ExecutorActorResult::JobRunning);
                }
            }
        }
        Ok(ExecutorActorResult::Ok)
    }
}
