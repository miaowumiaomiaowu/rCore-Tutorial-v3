use crate::task::{
    suspend_current_and_run_next,
    exit_current_and_run_next,
    TaskInfo, TaskStatus, SyscallInfo, MAX_SYSCALL_NUM, TASK_MANAGER
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

// pub fn sys_get_time() -> isize {
//     get_time_ms() as isize
// }

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// 系统调用函数实现
pub fn sys_task_info(id: usize, ts: *mut TaskInfo) -> isize {
    // 检查指针是否为空
    if ts.is_null() {
        return -1;
    }

    // 使用临时变量存储结果，避免长时间持有锁
    let mut info = TaskInfo {
        id: 0,
        status: TaskStatus::UnInit,
        call: [SyscallInfo { id: 0, times: 0 }; MAX_SYSCALL_NUM],
        time: 0,
    };
    
    {
        let inner = TASK_MANAGER.inner.exclusive_access();
        
        // 检查任务ID是否有效
        if id >= inner.tasks.len() || inner.tasks[id].task_status == TaskStatus::UnInit {
            return -1;
        }
        
        let task = &inner.tasks[id];
        
        // 计算实时运行时间
        let current_time = if task.task_status == TaskStatus::Running {
            task.total_time + (crate::timer::get_time() - task.start_time)
        } else {
            task.total_time
        };
        
        // 填充任务信息
        info.id = id;
        info.status = task.task_status;
        info.time = current_time;
        
        // 填充系统调用信息
        for i in 0..MAX_SYSCALL_NUM {
            if task.syscall_times[i] > 0 {
                info.call[i] = SyscallInfo {
                    id: i,
                    times: task.syscall_times[i],
                };
            }
        }
    }
    
    // 将信息写入用户指定的内存位置
    unsafe {
        *ts = info;
    }
    
    0
}