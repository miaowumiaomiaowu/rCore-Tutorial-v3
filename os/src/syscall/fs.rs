const FD_STDOUT: usize = 1;

// 添加一个函数来检查内存地址的有效性
fn check_valid_buffer(buf: *const u8, len: usize) -> bool {
    // 简单检查：确保缓冲区不为null且不在保留的低地址区域
    let buf_addr = buf as usize;
    if buf_addr == 0 || buf_addr < 0x1000 {
        return false;
    }
    // 更复杂的检查可以添加在这里，例如检查地址是否在用户空间范围内
    // ...
    true
}

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            // 首先检查缓冲区的有效性
            if !check_valid_buffer(buf, len) {
                return -1;
            }
            
            // 安全地读取缓冲区内容
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = match core::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            print!("{}", str);
            len as isize
        },
        _ => {
            -1 // 返回-1表示错误，而不是panic
        }
    }
}