#![feature(naked_functions)]
#![feature(llvm_asm)]

#[cfg(feature = "kiwami2")]
mod kiwami2;

#[cfg(not(feature = "kiwami2"))]
mod zero;

fn main() {
    #[cfg(feature = "kiwami2")]
    let result = kiwami2::main();

    #[cfg(not(feature = "kiwami2"))]
    let result = zero::main();

    // Slightly nicer way to print exit codes
    std::process::exit(match result {
        Ok(_) => 0,
        Err(error) => {
            eprintln!("Error: {}", error);
            1
        }
    })
}
