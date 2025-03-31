use super::TaskContext;
use super::MAX_SYSCALL_NUM;

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    // 新增字段
    pub syscall_times: [usize; MAX_SYSCALL_NUM],  // 记录每个系统调用的次数
    pub start_time: usize,                       // 任务开始时间
    pub total_time: usize,                       // 任务总运行时间
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}