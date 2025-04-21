//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

///A array of `TaskControlBlock` that is thread-safe
/// 第一部分：结构定义
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    /// ready_queue是一个双端队列，用来装载当前就绪状态的任务（任务控制块）
    /// Arc<TaskControlBlock>：每个任务的控制块是 TaskControlBlock，被 Arc 包裹说明它可以在多线程之间共享（线程安全引用计数智能指针）
}

/// A simple FIFO scheduler.
/// 第二部分：实现FIFO调度器逻辑
impl TaskManager {
    ///Creat an empty TaskManager
    /// 初始化一个空的队列
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    ///Add a task to `TaskManager`
    /// 把任务加入队尾（先进先出）
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    ///Remove the first task and return it,or `None` if `TaskManager` is empty
    /// 从队首取出下一个任务执行
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

/// 第三部分：全局单例定义
lazy_static! {//使用 lazy_static 宏来定义一个全局唯一的任务管理器
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> = //它被 UPSafeCell 包裹以实现安全的可变访问（用户态不会并发抢占）
        unsafe { UPSafeCell::new(TaskManager::new()) };
}
///Interface offered to add task
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}
///Interface offered to pop the first task
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
