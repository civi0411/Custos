//! T6 Bounded Repair Loop Topology Coordinator (RFC 004 §2A)
//!
//! Runs iterative Executor -> Evaluator repair cycles up to a strictly bounded maximum
//! number of iterations, preventing runaway retry loops.

use custos_core::oi::join_policy::{JoinEvaluation, JoinEvaluator, JoinPolicy};
use custos_domain::oi::WorkerResult;

pub struct T6RepairLoopCoordinator;

impl T6RepairLoopCoordinator {
    /// Evaluates an iteration of the repair loop.
    pub fn evaluate_iteration(
        max_iterations: u32,
        current_iteration: u32,
        evaluator_result: &WorkerResult,
    ) -> JoinEvaluation {
        let policy = JoinPolicy::BoundedIterations {
            max_iterations,
            current_iteration,
        };

        JoinEvaluator::evaluate(&policy, std::slice::from_ref(evaluator_result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_t6_repair_loop_bounded_retry() {
        let failure_res = WorkerResult::failed("eval_1", "Unit tests failed: assert_eq!(2, 3)");
        let eval = T6RepairLoopCoordinator::evaluate_iteration(3, 1, &failure_res);
        match eval {
            JoinEvaluation::RetryRequired { next_iteration, feedback } => {
                assert_eq!(next_iteration, 2);
                assert!(feedback.contains("assert_eq"));
            }
            _ => panic!("Expected retry required"),
        }

        // Exhaustion at max iterations
        let eval_exhausted = T6RepairLoopCoordinator::evaluate_iteration(3, 3, &failure_res);
        assert!(matches!(eval_exhausted, JoinEvaluation::Rejected { .. }));

        // Success
        let success_res = WorkerResult::success("eval_2", "All 15 tests passed");
        let eval_success = T6RepairLoopCoordinator::evaluate_iteration(3, 2, &success_res);
        assert!(matches!(eval_success, JoinEvaluation::Approved { .. }));
    }
}
