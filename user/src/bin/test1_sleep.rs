// 通过循环调用 get_time 和 yield_ 来模拟等待一段时间，间接测试 get_time 是否能返回递增的值。
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_time, yield_};

/// 正确输出：（无报错信息）
/// get_time OK! {...}
/// Test sleep OK!

#[no_mangle]
fn main() -> i32 {
    let current_time = get_time();//调用get_time获取当前时间
    assert!(current_time > 0);
    println!("get_time OK! {}", current_time);
    let wait_for = current_time + 3000;
    while get_time() < wait_for {
        yield_();
    }
    println!("Test sleep OK!");
    0
}
