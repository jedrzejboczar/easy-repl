use std::path::PathBuf;
use std::net::IpAddr;

use easy_repl::{Repl, CommandStatus, command};
use anyhow::{self, Context};

fn main() -> anyhow::Result<()> {
    let mut repl = Repl::builder()
        .add("ls", command! {
            "List files in a directory";
            (dir: PathBuf) => |dir: PathBuf| {
                for entry in dir.read_dir()? {
                    println!("{}", entry?.path().to_string_lossy());
                }
                Ok(CommandStatus::Done)
            }
        })
        .add("ipaddr", command! {
            "Just parse and print the given IP address";
            (ip: IpAddr) => |ip: IpAddr| {
                println!("{}", ip);
                Ok(CommandStatus::Done)
            }
        })
        .build().context("Failed to create repl")?;

    repl.run().context("Critical REPL error")
}

