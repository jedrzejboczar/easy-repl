use anyhow::{self, Context};
use easy_repl::{command, CommandStatus, Repl};

fn main() -> anyhow::Result<()> {
    #[rustfmt::skip]
    let mut repl = Repl::builder()
        .add("hello", command! {
            "Say hello",
            (name: String) => |name| {
                println!("Hello {}!", name);
                Ok(CommandStatus::Done)
            }
        })
        .add("add", command! {
            "Add X to Y",
            (X:i32, Y:i32) => |x, y| {
                println!("{} + {} = {}", x, y, x + y);
                Ok(CommandStatus::Done)
            }
        })
        .build()
        .context("Failed to create repl")?;

    repl.run().context("Critical REPL error")?;

    Ok(())
}
