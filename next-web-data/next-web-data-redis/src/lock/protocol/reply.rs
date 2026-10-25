//! Interpretation of the values the lock scripts return.

use redis::Value;

use crate::lock::error::RedisLockError;

/// A script reply the lock layer knows how to interpret.
///
/// The five lock scripts only ever return `nil`, a single integer, or an array
/// of integers, so the reply type is deliberately this small.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptReply {
    /// Lua `nil`, which the acquire script returns on success.
    Nil,
    /// A single integer.
    Int(i64),
    /// An array of integers, as returned by the status script.
    Ints(Vec<i64>),
}

impl ScriptReply {
    /// Interpret a raw Redis value.
    ///
    /// # Errors
    ///
    /// Returns [`RedisLockError::UnexpectedReply`] for any other shape, so a
    /// protocol mismatch surfaces instead of being silently coerced.
    pub fn parse(value: Value) -> Result<Self, RedisLockError> {
        match value {
            Value::Nil => Ok(Self::Nil),
            Value::Int(value) => Ok(Self::Int(value)),
            Value::Array(values) => {
                let mut ints = Vec::with_capacity(values.len());
                for value in values {
                    match value {
                        Value::Int(value) => ints.push(value),
                        other => {
                            return Err(RedisLockError::UnexpectedReply(format!(
                                "array element: {other:?}"
                            )));
                        }
                    }
                }
                Ok(Self::Ints(ints))
            }
            other => Err(RedisLockError::UnexpectedReply(format!("{other:?}"))),
        }
    }

    /// The single integer, when the reply is one.
    pub fn int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            Self::Nil | Self::Ints(_) => None,
        }
    }

    /// The integer array, when the reply is one.
    pub fn ints(&self) -> Option<&[i64]> {
        match self {
            Self::Ints(values) => Some(values),
            Self::Nil | Self::Int(_) => None,
        }
    }

    /// Whether the reply is Lua `nil`.
    pub fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nil_means_acquired() {
        assert_eq!(ScriptReply::parse(Value::Nil).unwrap(), ScriptReply::Nil);
        assert!(ScriptReply::parse(Value::Nil).unwrap().is_nil());
    }

    #[test]
    fn integers_and_arrays_round_trip() {
        assert_eq!(ScriptReply::parse(Value::Int(-1)).unwrap().int(), Some(-1));

        let reply = ScriptReply::parse(Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3000),
        ]))
        .unwrap();
        assert_eq!(reply.ints(), Some(&[1, 2, 3000][..]));
    }

    #[test]
    fn a_bulk_string_is_not_a_script_reply() {
        let error = ScriptReply::parse(Value::BulkString(b"nope".to_vec())).unwrap_err();
        assert!(matches!(error, RedisLockError::UnexpectedReply(_)));
    }
}
