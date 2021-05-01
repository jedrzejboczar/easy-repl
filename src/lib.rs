pub mod command;
pub mod shell;

pub use shell::Shell;
pub use command::{Command, CommandStatus};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
