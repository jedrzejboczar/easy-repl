pub mod command;
pub mod repl;
mod completion;

pub use repl::Repl;
pub use command::{Command, CommandStatus, CriticalError};

pub fn critical<E: Into<anyhow::Error>>(err: E) -> CriticalError {
    CriticalError::Critical(err.into())
}
