mod context;

use riscv::register::{
    mtvec::TrapMode,
    stvec,
    scause::{
        self,
        Trap,
        Exception,
        Interrupt,
    },
    stval,
    sie,
};
use crate::syscall::syscall;
use crate::task::{
    exit_current_and_run_next,
    suspend_current_and_run_next,
    MAX_SYSCALL_NUM
};
use crate::timer::set_next_trigger;
use crate::task::TASK_MANAGER;
use core::arch::global_asm;

global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" { fn __alltraps(); }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer(); }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();//读取异常原因
    let stval = stval::read();//读取错误地址
    match scause.cause() {//根据异常原因进行匹配
        Trap::Exception(Exception::UserEnvCall) => {//系统调用异常
            cx.sepc += 4;//更新程序计数器
            // 处理系统调用并记录调用次数
            let syscall_id = cx.x[17];//获取系统调用ID
            // 记录系统调用次数
            if syscall_id < MAX_SYSCALL_NUM {
                // 使用临时作用域确保锁被及时释放
                {
                    let mut inner = TASK_MANAGER.inner.exclusive_access();
                    let current = inner.current_task;
                    inner.tasks[current].syscall_times[syscall_id] += 1;
                }
            }
            // 更新系统调用返回值
            cx.x[10] = syscall(syscall_id, [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) |//存储错误
        Trap::Exception(Exception::StorePageFault) => {//存储页错误
            println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.", stval, cx.sepc);
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::LoadFault) |//加载错误
        Trap::Exception(Exception::LoadPageFault) => {//加载页错误
            println!("[kernel] LoadFault in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.", stval, cx.sepc);
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {//非法指令
            println!("[kernel] IllegalInstruction in application, core dumped.");
            exit_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {//时钟中断
            set_next_trigger();//设置下一个触发时间
            suspend_current_and_run_next();//挂起当前任务并运行下一个任务
        }
        _ => {//其他异常
            panic!("Unsupported trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    cx
}

pub use context::TrapContext;