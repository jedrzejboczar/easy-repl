use anyhow::{self, Context};
use easy_repl::{command, CommandStatus, Repl};

fn matryoshka(name: String) -> anyhow::Result<Repl<'static>> {
    let prompt = format!("{}> ", name);

    let cloned_prompt = prompt.clone(); // need to move it into closure
    let new = command! {
        "Enter new repl",
        (name:String) => |name: String| {
            let name = cloned_prompt.clone() + &name;
            let mut repl = matryoshka(name)?;
            repl.run()?;
            Ok(CommandStatus::Done)
        }
    };

    let repl = Repl::builder().prompt(prompt).add("new", new).build()?;

    Ok(repl)
}

fn main() -> anyhow::Result<()> {
    let mut repl = matryoshka("".into())?;
    repl.run().context("Critical REPL error")?;
    Ok(())
}
