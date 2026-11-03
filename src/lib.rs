//! retroforge: retrospectives that end in a pull
//! request. It reads a directory of flighttape
//! tapes, finds the moments where the operator's
//! fleet repeated a failed call or the operator
//! stepped in, and emits one proposal file citing
//! those moments (RF-1 to RF-14).
//!
//! Round 3 is the Rust port. Tape JSON reading and
//! the canonical value model are consumed from the
//! flighttape crate at a pinned revision (RF-16),
//! never re-implemented here. Canonical form 1
//! comes from the same pin; the digest module
//! renders it with the Python digest's float digit
//! source, because the crate's renderer is known to
//! differ there (RF-18).

pub mod cli;
pub mod digest;
pub mod emit;
pub mod evidence;
pub mod helptext;
pub mod moments;
pub mod proposals;
pub mod pyrepr;
pub mod repr_table;
pub mod tapes;
pub mod util;

/// The command line entry, in the cli module.
pub fn run_cli() -> i32 {
    // A generous stack, on the flighttape crate's own
    // precedent: parsed values nest as deep as their
    // files, and cloning or dropping a deep value
    // recurses with its depth.
    let child = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(crate::cli::run_cli)
        .expect("the CLI thread starts");
    child.join().unwrap_or(1)
}
