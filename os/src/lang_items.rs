use core::panic::PanicInfo;
use crate::sbi::shutdown;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap_or(&format_args!(""))
        );
    } else {
        println!(
            "[kernel] Panicked: {}",
            info.message().unwrap_or(&format_args!(""))
        );
    }
    shutdown()
}
