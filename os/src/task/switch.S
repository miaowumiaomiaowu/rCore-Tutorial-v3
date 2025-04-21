.altmacro
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a1)
.endm
    .section .text
    .globl __switch
__switch:
    # __switch(
    #     current_task_cx_ptr: *mut TaskContext,  保存当前任务的上下文到 current_task_cx_ptr
    #     next_task_cx_ptr: *const TaskContext   加载新任务的上下文从 next_task_cx_ptr,然后跳到新任务继续执行！
    # )
    # 保存当前任务的内核堆栈
    sd sp, 8(a0)
    # 保存当前执行的 ra 和 s0~s11
    # 保存当前任务的上下文到 current_task_cx_ptr
    # a0: current_task_cx_ptr要保存当前上下文的地址
    sd ra, 0(a0)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr
    
    # 恢复下一次执行的 ra 和 s0~s11
    # 加载新任务的上下文从 next_task_cx_ptr
    # a1: next_task_cx_ptr要加载新上下文的地址
    ld ra, 0(a1)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    # 恢复下一个任务的内核堆栈
    ld sp, 8(a1)

    ret
    # 使用刚刚从新任务加载的ra寄存器，所以会跳转到新任务上次返回时的位置，就像任务接着
    # 运行一样（其实是继续执行 trap_return → 用户程序）

