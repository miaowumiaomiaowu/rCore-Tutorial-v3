mod context;
mod switch;
mod task;

use crate::loader::{get_num_app, get_app_data};
use crate::trap::{TrapContext, trap_handler};
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};
use alloc::vec::Vec;
use crate::mm::{VirtAddr, VPNRange, MapPermission, VirtPageNum};

pub use context::TaskContext;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(
                get_app_data(i),
                i,
            ));
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
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(
                &mut _unused as *mut _,
                next_task_cx_ptr,
            );
        }
        panic!("unreachable in run_first_task!");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
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

    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    fn get_current_trap_cx(&self) -> &mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(
                    current_task_cx_ptr,
                    next_task_cx_ptr,
                );
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    fn get_current_task_cx_ptr2(&self) -> *mut TaskContext {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        &inner.tasks[current].task_cx as *const TaskContext as *mut TaskContext
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

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

pub fn task_mmap(start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) -> isize {
    let vpn_range = VPNRange::new(start_va.floor(), end_va.ceil());
    
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let memory_set = &mut inner.tasks[current].memory_set;// 获取当前任务的memory_set
    
    /// 冲突检查： 遍历 MemorySet 中已有的 areas，检查请求的虚拟地址范围 [start_va, end_va) 
    /// 是否与任何现有 MapArea 的 vpn_range 有重叠。如果存在重叠，返回 -1。
    let mut has_mapped = false;
    for vpn in vpn_range {
        for area in memory_set.areas.iter() {
            if area.vpn_range.contains(vpn) {
                has_mapped = true;
                break;
            }
        }
        if has_mapped {
            break;
        }
    }
    
    if has_mapped {
        return -1;
    }
    
    /// 插入新区域： 如果没有冲突，调用 memory_set.insert_framed_area(start_va, end_va, permission)。
    memory_set.insert_framed_area(start_va, end_va, permission);
    0
}

pub fn task_munmap(start_va: VirtAddr, end_va: VirtAddr) -> isize {
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let memory_set = &mut inner.tasks[current].memory_set;
    
    // 获取页范围
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();
    
    // 检查是否有匹配的区域
    let mut found_area = false;
    for (_, area) in memory_set.areas.iter().enumerate() {
        if area.vpn_range.get_start() == start_vpn && 
           area.vpn_range.get_end() == end_vpn {
            found_area = true;
            break;
        }
    }
    
    if !found_area {
        return -1;
    }
    
    // 找到完全匹配的区域，移除它
    let result = memory_set.remove_area_with_start_vpn(start_vpn);
    if result.is_some() {
        0
    } else {
        -1
    }
}

pub fn task_have_mapped(vpn: VirtPageNum) -> bool {
    let inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    inner.tasks[current].memory_set.translate(vpn).is_some()
}