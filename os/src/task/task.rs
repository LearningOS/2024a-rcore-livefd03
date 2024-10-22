//! Types related to task management

use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// start time of task
    pub start_time: usize,
}

impl Default for TaskControlBlock {
    fn default() -> Self {
        TaskControlBlock { 
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit, 
            syscall_times: [0;MAX_SYSCALL_NUM],
            start_time:0
        }
    }
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
