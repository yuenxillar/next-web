use std::any::Any;
use std::collections::HashSet;

use crate::state_machine::state::pseudo_state_kind::PseudoStateKind;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::StateContext;

pub const HEADER_DO_ACTION_TIMEOUT: &str = "STATEMACHINE_DO_ACTION_TIMEOUT";

/// 状态机工具类
pub struct StateMachineUtils;

impl StateMachineUtils {
    /// 检查 right 是否是 left 的子状态
    pub fn is_substate<S, E>(
        left: &dyn StateMachineState<S, E>,
        right: &dyn StateMachineState<S, E>,
    ) -> bool
    where
        S: PartialEq,
        S: Send + Sync,
        S: 'static,
        E: Send + Sync,
        E: 'static,
    {
        let states = left.states();
        // 移除自身（如果有）
        let filtered: Vec<_> = states.into_iter().filter(|s| s.id() != left.id()).collect();

        filtered.iter().any(|s| s.id() == right.id())
    }

    /// 检查 right 集合是否包含至少一个 left 集合中的元素
    pub fn contains_at_least_one<T: PartialEq>(left: &[T], right: &[T]) -> bool {
        if left.is_empty() || right.is_empty() {
            return false;
        }
        left.iter().any(|item| right.contains(item))
    }

    /// 检查状态是否为普通的伪状态（非 INITIAL 或 END）
    pub fn is_normal_pseudo_state<S, E>(state: &dyn StateMachineState<S, E>) -> bool
    where
        S: Send + Sync,
        S: 'static,
        E: Send + Sync,
        E: 'static,
    {
        let pseudo_state = match state.pseudo_state() {
            Some(ps) => ps,
            None => return false,
        };

        let kind = pseudo_state.get_kind();
        !matches!(kind, PseudoStateKind::Initial | PseudoStateKind::End)
    }

    /// 检查状态是否为瞬态伪状态（RTC 完成后不会停留）
    pub fn is_transient_pseudo_state<S, E>(state: &dyn StateMachineState<S, E>) -> bool
    where
        S: Send + Sync,
        S: 'static,
        E: Send + Sync,
        E: 'static,
    {
        let pseudo_state = match state.pseudo_state() {
            Some(ps) => ps,
            None => return false,
        };

        let kind = pseudo_state.get_kind();
        matches!(
            kind,
            PseudoStateKind::Choice
                | PseudoStateKind::Junction
                | PseudoStateKind::Entry
                | PseudoStateKind::Exit
                | PseudoStateKind::HistoryDeep
                | PseudoStateKind::HistoryShallow
                | PseudoStateKind::Fork
                | PseudoStateKind::Join
        )
    }

    /// 检查状态是否为指定类型的伪状态
    pub fn is_pseudo_state_of_kind<S, E>(
        state: &dyn StateMachineState<S, E>,
        kind: PseudoStateKind,
    ) -> bool
    where
        S: Send + Sync,
        S: 'static,
        E: Send + Sync,
        E: 'static,
    {
        state
            .pseudo_state()
            .map(|ps| ps.get_kind() == kind)
            .unwrap_or(false)
    }

    /// 将集合转换为字符串集合
    pub fn to_string_collection<T: ToString>(collection: &[T]) -> Vec<String> {
        collection.iter().map(|item| item.to_string()).collect()
    }

    /// 将任意对象转换为字符串集合（处理数组）
    pub fn to_string_collection_from_any(object: &dyn Any) -> Vec<String> {
        let mut result = Vec::new();

        // 尝试处理数组类型
        if let Some(array) = object.downcast_ref::<Vec<&dyn Any>>() {
            for item in array {
                result.push(format!("{:?}", item));
            }
        } else {
            // 单个对象
            result.push(format!("{:?}", object));
        }

        result
    }

    /// 检查字符串集合是否包含指定字符串
    pub fn contains_at_least_one_equal_string(left: &[String], right: &str) -> bool {
        left.iter().any(|s| s == right)
    }

    /// 检查两个字符串集合是否有交集
    pub fn contains_at_least_one_equal_string_collection(
        left: &[String],
        right: &[String],
    ) -> bool {
        if left.is_empty() || right.is_empty() {
            return false;
        }

        let right_set: HashSet<_> = right.iter().collect();
        left.iter().any(|item| right_set.contains(item))
    }

    /// 从状态上下文中获取 DO_ACTION_TIMEOUT 消息头
    pub fn get_message_header_do_action_timeout<S, E>(
        context: &dyn StateContext<S, E>,
    ) -> Option<i64>
    where
        S: 'static,
        E: 'static,
    {
        context.message_headers().map(|headers| {
            headers
                .get(HEADER_DO_ACTION_TIMEOUT)
                .map(|val| val.as_number())
                .unwrap_or_default()
        })?
    }

    // /// 将错误存入 Reactor context 的工具函数
    // pub fn resume_error_to_context<F>(executor_exception_holder: &mut ExecutorErrorHolder) {
    //     executor_exception_holder.set_error(error);
    // }
}
