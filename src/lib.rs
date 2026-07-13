#![cfg_attr(not(test), no_std)]
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    unsafe_op_in_unsafe_fn,
    dead_code,
    unused_imports
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::wildcard_enum_match_arm,
    clippy::inline_always,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

pub mod address;
pub mod apu;
pub mod bus;
pub mod clock;
pub mod cpu;
pub mod interrupt;
pub mod ops;
pub mod ppu;
pub mod rom;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
