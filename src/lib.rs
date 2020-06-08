pub mod common;
// #[cfg(feature = "kiwami2")]
// mod kiwami2;
// 
// #[cfg(feature = "kiwami")]
// mod kiwami;
// 
// #[cfg(all(not(feature = "kiwami2"), not(feature = "kiwami")))]
// mod zero;
// 
// mod common;
// 
// fn main() {
//     #[cfg(feature = "kiwami2")]
//     let result = kiwami2::main();
// 
//     #[cfg(feature = "kiwami")]
//     let result = kiwami::main();
// 
//     #[cfg(all(not(feature = "kiwami2"), not(feature = "kiwami")))]
//     let result = zero::main();
// 
//     // Slightly nicer way to print exit codes
//     std::process::exit(match result {
//         Ok(_) => 0,
//         Err(error) => {
//             eprintln!("Error: {}", error);
//             1
//         }
//     })
// }
