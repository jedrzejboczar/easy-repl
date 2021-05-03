use just_repl::{Shell, CommandStatus, command, args_validator};
use anyhow::{self, Context};

fn matryoshka(name: String) -> anyhow::Result<Shell<'static>> {
    let prompt = format!("{}> ", name);

    let cloned_prompt = prompt.clone();  // need to move it into closure
    let new = command! {
        "Enter new shell",
        String:name => |(name, ): (String, )| {
            let name = cloned_prompt.clone() + &name;
            let mut shell = matryoshka(name).unwrap();
            shell.run();
            CommandStatus::Done
        }
    };

    let shell = Shell::builder()
        .prompt(prompt)
        .add("new", new)
        .build()?;

    Ok(shell)
}

fn main() -> anyhow::Result<()> {
    let mut shell = matryoshka("".into())?;
    shell.run();
    Ok(())
}

