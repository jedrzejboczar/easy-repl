use std::time::Instant;

use easy_repl::{Repl, CommandStatus, command, critical};
use anyhow::{self, Context};

// this could be any funcion returining Result with an error implementing Error
// here for simplicity we make use of the Other variant of std::io::Error
fn may_throw(description: String) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(std::io::ErrorKind::Other, description))
}

fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    let mut repl = Repl::builder()
        .add("ok", command! {
            "Run a command that just succeeds",
            => |()| Ok(CommandStatus::Done)
        })
        .add("error", command! {
            "Command with recoverable error handled by the REPL",
            String:text => |(text, )| {
                may_throw(text)?;
                Ok(CommandStatus::Done)
            },
        })
        .add("critical", command! {
            "Command returns a critical error that must be handled outside of REPL",
            String:text => |(text, )| {
                // Short notation:
                may_throw(text).map_err(critical)?;
                // More explicitly it could be:
                //   if let Err(err) = may_throw(text) {
                //       Err(critical(err))?;
                //   }
                // or even:
                //   if let Err(err) = may_throw(text) {
                //       return Err(critical(err)).into();
                //   }
                Ok(CommandStatus::Done)
            },
        })
        .add("roulette", command! {
            "Feeling lucky?",
            => |()| {
                let ns = Instant::now().duration_since(start).as_nanos();
                let cylinder = ns % 6;
                match cylinder {
                    0 => may_throw("Bang!".into()).map_err(critical)?,
                    1..=2 => may_throw("Blank cartridge?".into())?,
                    _ => (),
                }
                Ok(CommandStatus::Done)
            },
        })
        .build().context("Failed to create repl")?;

    repl.run().context("Critical REPL error")?;

    Ok(())
}

