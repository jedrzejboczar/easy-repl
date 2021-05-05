use std::time::Instant;

use easy_repl::{Repl, CommandStatus, Critical, command};
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
            "Run a command that just succeeds";
            => || Ok(CommandStatus::Done)
        })
        .add("error", command! {
            "Command with recoverable error handled by the REPL";
            text:String => |text| {
                may_throw(text)?;
                Ok(CommandStatus::Done)
            },
        })
        .add("critical", command! {
            "Command returns a critical error that must be handled outside of REPL";
            text:String => |text| {
                // Short notation using the Critical trait
                may_throw(text).into_critical()?;
                // More explicitly it could be:
                //   if let Err(err) = may_throw(text) {
                //       Err(easy_repl::CriticalError::Critical(err.into()))?;
                //   }
                // or even:
                //   if let Err(err) = may_throw(text) {
                //       return Err(easy_repl::CriticalError::Critical(err.into())).into();
                //   }
                Ok(CommandStatus::Done)
            },
        })
        .add("roulette", command! {
            "Feeling lucky?";
            => || {
                let ns = Instant::now().duration_since(start).as_nanos();
                let cylinder = ns % 6;
                match cylinder {
                    0 => may_throw("Bang!".into()).into_critical()?,
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

