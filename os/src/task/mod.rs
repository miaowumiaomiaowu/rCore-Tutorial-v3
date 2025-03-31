mod context;
mod switch;
mod task;

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use lazy_static::*;
use switch::__switch;
use task::TaskControlBlock;
use crate::sync::UPSafeCell;
use crate::timer::get_time;

pub use context::TaskContext;
pub use task::TaskStatus;

pub const MAX_SYSCALL_NUM: usize = 500; // 系统调用最大数量

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskInfo { // 定义一个结构体TaskInfo，用于存储任务信息
    pub id: usize, // 任务ID
    pub status: TaskStatus, // 任务状态
    pub call: [SyscallInfo; MAX_SYSCALL_NUM], // 系统调用信息数组
    pub time: usize // 任务运行时间
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct SyscallInfo { // 定义一个结构体SyscallInfo，用于存储系统调用信息
    pub id: usize, // 系统调用ID
    pub times: usize // 系统调用次数
}

pub struct TaskManager {
    pub num_app: usize,
    pub inner: UPSafeCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    pub tasks: [TaskControlBlock; MAX_APP_NUM],
    pub current_task: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [
            TaskControlBlock {
                task_cx: TaskContext::zero_init(),
                task_status: TaskStatus::UnInit,
                syscall_times: [0; MAX_SYSCALL_NUM],
                start_time: 0,
                total_time: 0,
            };
            MAX_APP_NUM
        ];
        for i in 0..num_app {
            tasks[i].task_cx = TaskContext::goto_restore(init_app_cx(i));
            tasks[i].task_status = TaskStatus::Ready;
            // 初始化系统调用计数和时间
            tasks[i].syscall_times = [0; MAX_SYSCALL_NUM];
            tasks[i].start_time = 0;
            tasks[i].total_time = 0;
        }
        TaskManager {
            num_app,
            inner: unsafe { UPSafeCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            })},
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        // 记录任务开始时间
        task0.start_time = get_time();
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(
                &mut _unused as *mut TaskContext,
                next_task_cx_ptr,
            );
        }
        panic!("unreachable in run_first_task!");
    }

    fn mark_current_suspended(&self) {
        // 使用一个临时作用域来限制锁的持有时间
        {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            // 记录任务运行时间
            let task = &mut inner.tasks[current];
            task.task_status = TaskStatus::Ready;
            // 更新任务累计运行时间
            task.total_time += get_time() - task.start_time;
        }
    }

    fn mark_current_exited(&self) {
        // 使用一个临时作用域来限制锁的持有时间
        {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            // 记录任务运行时间
            let task = &mut inner.tasks[current];
            task.task_status = TaskStatus::Exited;
            // 更新任务累计运行时间
            task.total_time += get_time() - task.start_time;
        }
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| {
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            // 记录任务开始时间
            inner.tasks[next].start_time = get_time();
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner); // 释放锁后再进行上下文切换
            // 切换到下一个任务
            unsafe {
                __switch(
                    current_task_cx_ptr,
                    next_task_cx_ptr,
                );
            }
        } else {
            panic!("All applications completed!");
        }
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn get_current_task_info() -> TaskInfo {
    let inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let task = &inner.tasks[current];
    
    let mut info = TaskInfo {
        id: current,
        status: task.task_status,
        call: [SyscallInfo { id: 0, times: 0 }; MAX_SYSCALL_NUM],
        time: task.total_time,
    };
    
    // 填充系统调用信息
    for i in 0..MAX_SYSCALL_NUM {
        if task.syscall_times[i] > 0 {
            info.call[i] = SyscallInfo {
                id: i,
                times: task.syscall_times[i],
            };
        }
    }
    
    info
}