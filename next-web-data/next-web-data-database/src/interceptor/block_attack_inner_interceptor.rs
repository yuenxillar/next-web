use rbatis::async_trait;
use rbatis::{
    executor::Executor,
    intercept::{Intercept, ResultType},
    rbdc::db::ExecResult,
    Error,
};
use rbs::Value;
use tracing::warn;

#[derive(Debug, Default)]
pub struct BlockAttackInnerInterceptor;

#[async_trait]
impl Intercept for BlockAttackInnerInterceptor {
    /// if return Some(false) will be break
    async fn before(
        &self,
        _task_id: i64,
        _rb: &dyn Executor,
        _sql: &mut String,
        _args: &mut Vec<Value>,
        _result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Vec<Value>, Error>>,
    ) -> Result<Option<bool>, Error> {
        let normalized_sql = _sql.trim().to_uppercase();

        // Check whether the full table is updated. If yes, exit without executing
        if normalized_sql.starts_with("UPDATE") && !normalized_sql.contains("WHERE") {
            warn!("Full table update detected, exit without executing");
            return Ok(Some(false));
        }

        // Check whether the full table is deleted. If yes, exit without executing
        if normalized_sql.starts_with("DELETE") && !normalized_sql.contains("WHERE") {
            warn!("Full table delete detected, exit without executing");
            return Ok(Some(false));
        }
        Ok(Some(true))
    }

    /// task_id maybe is conn_id or tx_id,
    /// is_prepared_sql = !args.is_empty(),
    /// if return Ok(false) will be return data. return Ok(true) will run next
    async fn after(
        &self,
        _task_id: i64,
        _rb: &dyn Executor,
        _sql: &mut String,
        _args: &mut Vec<Value>,
        _result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Vec<Value>, Error>>,
    ) -> Result<Option<bool>, Error> {
        Ok(Some(true))
    }
}
