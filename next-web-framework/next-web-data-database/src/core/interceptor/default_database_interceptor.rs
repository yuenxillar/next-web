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
pub struct DefaultDatabaseInterceptor;

#[async_trait]
impl Intercept for DefaultDatabaseInterceptor {
    /// if return Some(false) will be break
    async fn before(
        &self,
        _task_id: i64,
        _rb: &dyn Executor,
        _sql: &mut String,
        _args: &mut Vec<Value>,
        _result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Vec<Value>, Error>>,
    ) -> Result<Option<bool>, Error> {
        // Check whether the full table is updated. If yes, exit without executing
        if (_sql.starts_with("UPDATE") || _sql.starts_with("update"))
            && (!_sql.contains("WHERE") && !_sql.contains("where"))
        {
            warn!("Full table update detected, exit without executing");
            return Ok(Some(false));
        }

        // Check whether the full table is deleted. If yes, exit without executing
        if (_sql.starts_with("DELETE") || _sql.starts_with("delete"))
            && (!_sql.contains("WHERE") && !_sql.contains("where"))
        {
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
