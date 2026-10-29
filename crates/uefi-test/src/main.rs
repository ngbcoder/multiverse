#![no_std]
#![no_main]

use uefi::prelude::*;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Hello from UEFI test app!\r\n"));
    });
    uefi::boot::stall(5_000_000);
    Status::SUCCESS
}