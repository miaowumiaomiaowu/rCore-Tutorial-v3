// os/src/lang_items.rs
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!("Panicked at {}:{}",
            location.file(),
            location.line(),
        );
        if let Some(message) = info.message() {
            println!("{}", message);
        }
    } else {
        println!("Panicked");
        if let Some(message) = info.message() {
            println!("{}", message);
        }
    }
    loop {}
}