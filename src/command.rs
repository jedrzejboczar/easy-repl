//! Implementation of [`Command`]s with utilities that help to crate them.

use anyhow;
use thiserror;

/// Command handler.
///
/// It should return the status in case of correct execution. In case of
/// errors, all the errors will be handled by the REPL, except for
/// [`CriticalError`], which will be passed up from the REPL.
///
/// The handler should validate command arguments and can return [`ArgsError`]
/// to indicate that arguments were wrong.
pub type Handler<'a> = dyn 'a + FnMut(&[&str]) -> anyhow::Result<CommandStatus>;

/// Single command that can be called in the REPL.
///
/// Though it is possible to construct it by manually, it is not advised.
/// One should rather use the provided [`command!`] macro which will generate
/// appropriate arguments validation and args_info based on passed specification.
pub struct Command<'a> {
    /// Command desctiption that will be displayed in the help message
    pub description: String,
    /// Names and types of arguments to the command
    pub args_info: Vec<String>,
    /// Command handler which should validate arguments and perform command logic
    pub handler: Box<Handler<'a>>,
}

/// Return status of a command.
#[derive(Debug)]
pub enum CommandStatus {
    /// Indicates that REPL should continue execution
    Done,
    /// Indicates that REPL should quit
    Quit,
}

/// Special error wrapper used to indicate that a critical error occured.
///
/// [`Handler`] can return [`CriticalError`] to indicate that this error
/// should not be handled by the REPL (which just prints error message
/// and continues for all other errors).
///
/// This is most conveniently used via the [`Critical`] extension trait.
#[derive(Debug, thiserror::Error)]
pub enum CriticalError {
    /// The contained error is critical and should be returned back from REPL.
    #[error(transparent)]
    Critical(#[from] anyhow::Error),
}

/// Extension trait to easily wrap errors in [`CriticalError`].
///
/// This is implemented for [`std::result::Result`] so can be used to coveniently
/// wrap errors that implement [`std::error::Error`] to indicate that they are
/// critical and should be returned by the REPL, for example:
/// ```rust
/// # use easy_repl::{CriticalError, Critical};
/// let result: Result<(), std::fmt::Error> = Err(std::fmt::Error);
/// let critical = result.into_critical();
/// assert!(matches!(critical, Err(CriticalError::Critical(_))));
/// ```
///
/// See `examples/errors.rs` for a concrete usage example.
pub trait Critical<T, E> {
    /// Wrap the contained [`Err`] in [`CriticalError`] or leave [`Ok`] untouched
    fn into_critical(self) -> Result<T, CriticalError>;
}

impl<T, E> Critical<T, E> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_critical(self) -> Result<T, CriticalError> {
        self.map_err(|e| CriticalError::Critical(e.into()))
    }
}

/// Wrong command arguments.
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum ArgsError {
    #[error("wrong number of arguments: got {got}, expected {expected}")]
    WrongNumberOfArguments { got: usize, expected: usize },
    #[error("failed to parse argument value '{argument}': {error}")]
    WrongArgumentValue {
        argument: String,
        #[source]
        error: anyhow::Error,
    },
}

impl<'a> Command<'a> {
    /// Validate the arguments and invoke the handler if arguments are correct.
    pub fn run(&mut self, args: &[&str]) -> anyhow::Result<CommandStatus> {
        (self.handler)(args)
    }
}

impl<'a> std::fmt::Debug for Command<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("description", &self.description)
            .finish()
    }
}

/// Generate argument validator based on a list of types (used by [`command!`]).
///
/// This macro can be used to generate a closure that takes arguments as `&[&str]`
/// and makes sure that the nubmer of arguments is correct and all can be parsed
/// to appropriate types. This macro should generally not be used. Prefer to use
/// [`command!`] which will use this macro appropriately.
///
/// Example usage:
/// ```rust
/// # use easy_repl::validator;
/// let validator = validator!(i32, f32, String);
/// assert!(validator(&["10", "3.14", "hello"]).is_ok());
/// ```
///
/// # Note
///
/// For string arguments use [`String`] instead of [`&str`].
#[macro_export]
macro_rules! validator {
    ($($type:ty),*) => {
        |args: &[&str]| -> std::result::Result<(), $crate::command::ArgsError> {
            // check the number of arguments
            let n_args: usize = <[()]>::len(&[ $( $crate::validator!(@replace $type ()) ),* ]);
            if args.len() != n_args {
                return Err($crate::command::ArgsError::WrongNumberOfArguments {
                    got: args.len(),
                    expected: n_args,
            });
            }
            #[allow(unused_variables, unused_mut)]
            let mut i = 0;
            #[allow(unused_assignments)]
            {
                $(
                    if let Err(err) = args[i].parse::<$type>() {
                        return Err($crate::command::ArgsError::WrongArgumentValue {
                            argument: args[i].into(),
                            error: err.into()
                    });
                    }
                    i += 1;
                )*
            }

            Ok(())
        }
    };
    // Helper that allows to replace one expression with another (possibly "noop" one)
    (@replace $_old:tt $new:expr) => { $new };
}

// TODO: avoid parsing arguments 2 times by generating validation logic in the function
/// Generate [`Command`] based on desctiption, list of arg types and a closure used in handler.
///
/// This macro should be used when creating [`Command`]s. It takes a string description,
/// a list of argument types with optional names (in the form `name: type`) and a closure.
/// The closure should have the same number of arguments as provided in the argument list.
/// The generated command handler will parse all the arguments and call the closure.
/// The closure used for handler is `move`.
///
/// The following command description:
/// ```rust
/// # use easy_repl::{CommandStatus, command};
/// let cmd = command! {
///     "Example command";
///     arg1: i32, arg2: String => |arg1, arg2| {
///         Ok(CommandStatus::Done)
///     }
/// };
/// ```
///
/// will roughly be translated into something like (code here is slightly simplified):
/// ```rust
/// # use anyhow;
/// # use easy_repl::{Command, CommandStatus, command, validator};
/// let cmd = Command {
///     description: "Example command".into(),
///     args_info: vec!["arg1:i32".into(), "arg2:String".into()],
///     handler: Box::new(move |args| -> anyhow::Result<CommandStatus> {
///         let validator = validator!(i32, String);
///         validator(args)?;
///         let mut handler = |arg1, arg2| {
///             Ok(CommandStatus::Done)
///         };
///         handler(args[0].parse::<i32>().unwrap(), args[1].parse::<String>().unwrap())
///     }),
/// };
/// ```
#[macro_export]
macro_rules! command {
    ($description:expr; $($( $name:ident )? : $type:ty),* => $handler:expr $(,)?) => {
        $crate::command::Command {
            description: $description.into(),
            args_info: vec![ $(
                concat!($(stringify!($name), )? ":", stringify!($type)).into()
            ),* ], // TODO
            handler: command!(@handler $($type)*, $handler),
        }
    };
    (@handler $($type:ty)*, $handler:expr) => {
        Box::new( move |#[allow(unused_variables)] args| -> anyhow::Result<CommandStatus> {
            let validator = $crate::validator!($($type),*);
            validator(args)?;
            #[allow(unused_mut)]
            let mut handler = $handler;
            command!(@handler_call handler; args; $($type;)*)
        })
    };
    // transform element of $args into parsed function argument by calling .parse::<$type>().unwrap()
    // on each, this starts a recursive muncher that constructs following argument getters args[i]
    (@handler_call $handler:ident; $args:ident; $($types:ty;)*) => {
        command!(@handler_call $handler, $args, 0; $($types;)* =>)
    };
    // $num is used to index $args; pop $type from beginning of list, add new parsed at the endo of $parsed
    (@handler_call $handler:ident, $args:ident, $num:expr; $type:ty; $($types:ty;)* => $($parsed:expr;)*) => {
        command!(@handler_call $handler, $args, $num + 1;
            $($types;)* =>
            $($parsed;)* $args[$num].parse::<$type>().unwrap();
        )
    };
    // finally when there are no more types emit code that calls the handler with all arguments parsed
    (@handler_call $handler:ident, $args:ident, $num:expr; => $($parsed:expr;)*) => {
        $handler( $($parsed),* )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_command() {
        let mut cmd = Command {
            description: "Test command".into(),
            args_info: vec![],
            handler: Box::new(|_args| Ok(CommandStatus::Done)),
        };
        match (cmd.handler)(&[]) {
            Ok(CommandStatus::Done) => {}
            _ => panic!("Wrong variant"),
        };
    }

    #[test]
    fn validator_no_args() {
        let validator = validator!();
        assert!(validator(&[]).is_ok());
        assert!(validator(&["hello"]).is_err());
    }

    #[test]
    fn validator_one_arg() {
        let validator = validator!(i32);
        assert!(validator(&[]).is_err());
        assert!(validator(&["hello"]).is_err());
        assert!(validator(&["13"]).is_ok());
    }

    #[test]
    fn validator_multiple_args() {
        let validator = validator!(i32, f32, String);
        assert!(validator(&[]).is_err());
        assert!(validator(&["1", "2.1", "hello"]).is_ok());
        assert!(validator(&["1.2", "2.1", "hello"]).is_err());
        assert!(validator(&["1", "a", "hello"]).is_err());
        assert!(validator(&["1", "2.1", "hello", "world"]).is_err());
    }

    #[test]
    fn command_auto_no_args() {
        let mut cmd = command! {
            "Example cmd";
            => || {
                Ok(CommandStatus::Done)
            }
        };
        match cmd.run(&[]) {
            Ok(CommandStatus::Done) => {}
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => panic!("Error: {:?}", e),
        };
    }

    #[test]
    fn command_auto_with_args() {
        let mut cmd = command! {
            "Example cmd";
            :i32, :f32 => |_x, _y| {
                Ok(CommandStatus::Done)
            }
        };
        match cmd.run(&["13", "1.1"]) {
            Ok(CommandStatus::Done) => {}
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => panic!("Error: {:?}", e),
        };
    }

    #[test]
    fn command_auto_with_critical() {
        let mut cmd = command! {
            "Example cmd";
            :i32, :f32 => |_x, _y| {
                let err = std::io::Error::new(std::io::ErrorKind::InvalidData, "example error");
                Err(CriticalError::Critical(err.into()).into())
            }
        };
        match cmd.run(&["13", "1.1"]) {
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => {
                if e.downcast_ref::<CriticalError>().is_none() {
                    panic!("Wrong error: {:?}", e)
                }
            }
        };
    }

    #[test]
    fn command_auto_args_info() {
        let cmd = command!("Example cmd"; :i32, :String, :f32 => |_x, _s, _y| { Ok(CommandStatus::Done) });
        assert_eq!(cmd.args_info, &[":i32", ":String", ":f32"]);
        let cmd = command!("Example cmd"; :i32, :f32 => |_x, _y| { Ok(CommandStatus::Done) });
        assert_eq!(cmd.args_info, &[":i32", ":f32"]);
        let cmd = command!("Example cmd"; :f32 => |_x| { Ok(CommandStatus::Done) });
        assert_eq!(cmd.args_info, &[":f32"]);
        let cmd = command!("Example cmd"; => || { Ok(CommandStatus::Done) });
        let res: &[&str] = &[];
        assert_eq!(cmd.args_info, res);
    }

    #[test]
    fn command_auto_args_info_with_names() {
        let cmd = command! {
            "Example cmd";
            number:i32, name : String, :f32 => |_x, _s, _y| { Ok(CommandStatus::Done) }
        };
        assert_eq!(cmd.args_info, &["number:i32", "name:String", ":f32"]);
    }
}
