use thiserror;
use anyhow;

/// Represents a sign
pub struct Command<'a> {
    pub description: String,
    pub args_info: Vec<String>,
    pub handler: Box<dyn 'a + FnMut(&[&str]) -> anyhow::Result<CommandStatus>>,
    pub validator: Box<dyn FnMut(&[&str]) -> Result<(), ArgsError>>,
}

#[derive(Debug)]
pub enum CommandStatus {
    Done,
    Quit,
}

#[derive(Debug, thiserror::Error)]
pub enum CriticalError {
    #[error(transparent)]
    Critical(#[from] anyhow::Error)
}

#[derive(Debug, thiserror::Error)]
pub enum ArgsError {
    #[error("wrong number of arguments: got {0}, expected {1}")]
    WrongNumberOfArguments(usize, usize),
    #[error("failed to parse argument value '{0}'")]
    WrongArgumentValue(String, #[source] anyhow::Error),
}

impl<'a> Command<'a> {
    pub fn run(&mut self, args: &[&str]) -> anyhow::Result<CommandStatus> {
        (self.validator)(args)?;
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

/// Takes a list of types and generates a callable that will take a list
/// of args and try to parse them into the provided types or return an error.
/// NOTE: for string arguments use String instead of &str
#[macro_export]
macro_rules! args_validator {
    ($($type:ty)*) => {
        |args: &[&str]| -> std::result::Result<(), $crate::command::ArgsError> {
            // check the number of arguments
            let n_args: usize = <[()]>::len(&[ $( $crate::args_validator!(@replace $type ()) ),* ]);
            if args.len() != n_args {
                return Err($crate::command::ArgsError::WrongNumberOfArguments(args.len(), n_args));
            }
            #[allow(unused_variables, unused_mut)]
            let mut i = 0;
            #[allow(unused_assignments)]
            {
                $(
                    if let Err(err) = args[i].parse::<$type>() {
                        return Err($crate::command::ArgsError::WrongArgumentValue(args[i].into(), err.into()));
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

/// Creates a Command based on description, list of argument types and a command handler.
/// The command handler should be a lambda that will take all the arguments as a single tuple
/// and return CommandStatus.
/// NOTE: even if there are no arguments it must take an empty tuple
/// TODO: should there be an option to use closure with `move`?
/// TODO: find a way to avoid that tuple, is this even possible without procedural macros?
/// Additional glue logic that parses argument strings into concrete types will be added.
/// Also, an argument validator will be auto-generated based on provided types.
#[macro_export]
macro_rules! command {
    ($description:expr, $($type:ty $(: $name:ident)?)* => $handler:expr $(,)?) => {
        $crate::command::Command {
            description: $description.into(),
            args_info: vec![ $(
                concat!($(stringify!($name), )? ":", stringify!($type)).into()
            ),* ], // TODO
            validator: std::boxed::Box::new($crate::args_validator!( $($type)* )),
            handler: command!(@handler $($type)*, $handler),
        }
    };
    (@handler $($type:ty)*, $handler:expr) => {
        Box::new( move |#[allow(unused_variables)] args| -> anyhow::Result<CommandStatus> {
            let tuple_args: ($($type,)*) = command!(@tuple args; $($type;)*);
            #[allow(unused_mut)]
            let mut handler = $handler;
            handler(tuple_args)
        })
    };
    // transform element of Vec $args into tuple elements calling .parse::<$type>().unwrap() on each
    (@tuple $args:ident; $($types:ty;)*) => {
        command!(@tuple $args, 0;
            $($types;)* =>
        )
    };
    // $num is used to index $args, pop $type from beginning of list, add new parsed at the endo of $parsed
    (@tuple $args:ident, $num:expr; $type:ty; $($types:ty;)* => $($parsed:expr;)*) => {
        command!(@tuple $args, $num + 1;
            $($types;)* =>
            $($parsed;)* $args[$num].parse::<$type>().unwrap();
        )
    };
    (@tuple $args:ident, $num:expr; => $($parsed:expr;)*) => {  // finally emit code when there are no more types
        ( $($parsed,)* )
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
            handler: Box::new(|_args| {
                Ok(CommandStatus::Done)
            }),
            validator: Box::new(|_args| {
                Ok(())
            }),
        };
        match (cmd.handler)(&[]) {
            Ok(CommandStatus::Done) => {},
            _ => panic!("Wrong variant")
        };
    }

    #[test]
    fn validator_no_args() {
        let validator = args_validator!();
        assert!(validator(&[]).is_ok());
        assert!(validator(&["hello"]).is_err());
    }

    #[test]
    fn validator_one_arg() {
        let validator = args_validator!(i32);
        assert!(validator(&[]).is_err());
        assert!(validator(&["hello"]).is_err());
        assert!(validator(&["13"]).is_ok());
    }

    #[test]
    fn validator_multiple_args() {
        let validator = args_validator!(i32 f32 String);
        assert!(validator(&[]).is_err());
        assert!(validator(&["1", "2.1", "hello"]).is_ok());
        assert!(validator(&["1.2", "2.1", "hello"]).is_err());
        assert!(validator(&["1", "a", "hello"]).is_err());
        assert!(validator(&["1", "2.1", "hello", "world"]).is_err());
    }

    #[test]
    fn command_auto_no_args() {
        let mut cmd = command! {
            "Example cmd",
            => |()| {
                Ok(CommandStatus::Done)
            }
        };
        match cmd.run(&[]) {
            Ok(CommandStatus::Done) => {},
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => panic!("Error: {:?}", e),
        };
    }

    #[test]
    fn command_auto_with_args() {
        let mut cmd = command! {
            "Example cmd",
            i32 f32 => |(_x, _y)| {
                Ok(CommandStatus::Done)
            }
        };
        match cmd.run(&["13", "1.1"]) {
            Ok(CommandStatus::Done) => {},
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => panic!("Error: {:?}", e),
        };
    }

    #[test]
    fn command_auto_with_failure() {
        let mut cmd = command! {
            "Example cmd",
            i32 f32 => |(_x, _y)| {
                let err = std::io::Error::new(std::io::ErrorKind::InvalidData, "example error");
                Err(CriticalError::Critical(err.into()).into())
            }
        };
        match cmd.run(&["13", "1.1"]) {
            Ok(v) => panic!("Wrong variant: {:?}", v),
            Err(e) => {
                if e.downcast_ref::<CriticalError>().is_some() {
                    panic!("Wrong error: {:?}", e)
                }
            },
        };
    }

    #[test]
    fn command_auto_args_info() {
        let cmd = command!("Example cmd", i32 String f32 => |(_x, _s, _y)| { Ok(CommandStatus::Done) });
        assert_eq!(cmd.args_info, &[":i32", ":String", ":f32"]);
        let cmd = command!("Example cmd", i32 f32 => |(_x, _y)| { Ok(CommandStatus::Done) });
        assert_eq!(cmd.args_info, &[":i32", ":f32"]);
        let cmd = command!("Example cmd", f32 => |(_x,)| { Ok(CommandStatus::Done) });
        assert_eq!(cmd.args_info, &[":f32"]);
        let cmd = command!("Example cmd", => |()| { Ok(CommandStatus::Done) });
        let res: &[&str] = &[];
        assert_eq!(cmd.args_info, res);
    }

    #[test]
    fn command_auto_args_info_with_names() {
        let cmd = command! {
            "Example cmd",
            i32:number String : name f32 => |(_x, _s, _y)| { Ok(CommandStatus::Done) }
        };
        assert_eq!(cmd.args_info, &["number:i32", "name:String", ":f32"]);
    }
}
