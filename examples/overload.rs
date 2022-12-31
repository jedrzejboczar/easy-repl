use anyhow::{self, Context};
use easy_repl::{command, CommandStatus, Repl};

fn main() -> anyhow::Result<()> {
    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("describe", command! {
            "Variant 1",
            () => || {
                println!("No arguments");
                Ok(CommandStatus::Done)
            }
        })
        .add("describe", command! {
            "Variant 2",
            (a: i32, b: i32) => |a, b| {
                println!("Got two integers: {} {}", a, b);
                Ok(CommandStatus::Done)
            }
        })
        .add("describe", command! {
            "Variant 3",
            (a: i32, b: String) => |a, b| {
                println!("An integer `{}` and a string `{}`", a, b);
                Ok(CommandStatus::Done)
            }
        })
        .build()
        .context("Failed to create repl")?;

    repl.run().context("Critical REPL error")?;

    Ok(())
}
