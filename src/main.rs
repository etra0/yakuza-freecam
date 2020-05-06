#![feature(naked_functions)]
#![feature(llvm_asm)]
mod zero;
mod kiwami2;

fn main() {
    if cfg!(feature = "kiwami2") {
        kiwami2::main();
    } else {
        zero::main();
    }
}
