use std::sync::Arc;

use axum::{Json, Router, extract::State, response::IntoResponse, routing::post};

use crate::{
    executor::{
        context::job_context::JobContext,
        models::{ExecutorActorReq, ExecutorActorResult},
    },
    web_server::{
        middleware::verify_token::verify_token_middleware,
        models::{
            api_model::{JobIdleBeatParam, JobRunParam},
            api_result::XxlApiResult,
        },
        state::XxlJobAppState,
    },
};

pub(crate) fn app(state: Arc<XxlJobAppState>) -> Router {
    Router::new()
        .route("/beat", post(beat))
        .route("/idleBeat", post(idle_beat))
        .route("/run", post(run))
        .route("/kill", post(kill))
        .route("/log", post(log))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            verify_token_middleware,
        ))
        .with_state(state)
}

async fn beat() -> impl IntoResponse {
    XxlApiResult::<()>::success(None)
}

async fn idle_beat(
    State(state): State<Arc<XxlJobAppState>>,
    Json(param): Json<JobIdleBeatParam>,
) -> impl IntoResponse {
    if let Ok(ExecutorActorResult::Ok) = state
        .executor_actor
        .send(ExecutorActorReq::IdleBeat {
            job_id: param.job_id,
        })
        .await
    {
        XxlApiResult::<()>::success(None)
    } else {
        XxlApiResult::fail(Some("job is running or has trigger queue.".to_string()))
    }
}

async fn run(
    State(state): State<Arc<XxlJobAppState>>,
    Json(param): Json<JobRunParam>,
) -> impl IntoResponse {
    // let job_name = param.executor_handler.as_ref();

    if param
        .executor_handler
        .as_ref()
        .map(|s| s.is_empty())
        .unwrap_or(true)
    {
        return XxlApiResult::fail(Some(format!(
            "executor_handler is empty,log_id:{}",
            param.log_id
        )));
    }

    let job_name = param.executor_handler.clone().unwrap_or_default();
    let job_context = JobContext::new(param, state.clone());

    let _ = state
        .executor_actor
        .send(ExecutorActorReq::RunJob {
            job_name,
            job_context,
        })
        .await;

    XxlApiResult::<()>::success(None)
}

async fn kill(Json(_param): Json<JobIdleBeatParam>) -> impl IntoResponse {
    XxlApiResult::<()>::success(None)
}

async fn log() -> impl IntoResponse {
    XxlApiResult::<()>::success(None)
}
