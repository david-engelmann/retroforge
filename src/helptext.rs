//! The argparse help and usage texts, captured
//! from the Python at COLUMNS=80 and pinned byte
//! for byte (RF-19), each command's help and its
//! usage line.

pub const TOP_HELP: &str = "usage: retroforge [-h] {propose} ...\n\nRetrospectives that end in a pull request\n\npositional arguments:\n  {propose}\n    propose   propose edits from a directory of tapes\n\noptions:\n  -h, --help  show this help message and exit\n";
pub const TOP_USAGE: &str = "usage: retroforge [-h] {propose} ...";

pub const PROPOSE_HELP: &str = "usage: retroforge propose [-h] [--target TARGET] [-o OUTPUT] source_dir\n\npositional arguments:\n  source_dir\n\noptions:\n  -h, --help            show this help message and exit\n  --target TARGET\n  -o OUTPUT, --output OUTPUT\n";
pub const PROPOSE_USAGE: &str =
    "usage: retroforge propose [-h] [--target TARGET] [-o OUTPUT] source_dir";
