#![deny(missing_docs)]

//! An easy to use REPL, ideal when there is a need to crate an ad-hoc shell.
//!
//! This library provides a fast and convenient way to generate a
//! [REPL](https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop)
//! for your application. It comes with easy to use [`command!`] macro that
//! will automatically validate and parse command arguments, doing all the type
//! checking for you. The REPL comes with handy help messages, input validation,
//! hints and TAB-completion. Many REPL features can be configured.
//!
//! # Example
//!
//! This is a basic example corresponding to `examples/minimal.rs`. For more examples
//! see the `examples/` directory, which among others shows how to handle errors, access
//! variables outside of handler closures and how to create REPL inside REPL, inside REPL, inside...
//!
//! ```rust
//! use easy_repl::{Repl, CommandStatus, command};
//!
//! let mut repl = Repl::builder()
//!     .add("hello", command! {
//!         "Say hello",
//!         (name: String) => |name| {
//!             println!("Hello {}!", name);
//!             Ok(CommandStatus::Done)
//!         }
//!     })
//!     .add("add", command! {
//!         "Add X to Y",
//!         (X:i32, Y:i32) => |x, y| {
//!             println!("{} + {} = {}", x, y, x + y);
//!             Ok(CommandStatus::Done)
//!         }
//!     })
//!     .build().expect("Failed to create repl");
//!
//! repl.run().expect("Critical REPL error");
//! ```
//!
//! The generated REPL can be used as:
//! ```text
//! > hello world
//! Hello world!
//! ```
//!
//! It comes with argument number checking...
//! ```text
//! > add 1
//! Error: wrong number of arguments: got 1, expected 2
//! Usage: add X:i32 Y:i32
//! > hello easy repl
//! Error: wrong number of arguments: got 2, expected 1
//! Usage: hello name:String
//! > hello "easy repl"
//! Hello easy repl!
//! ```
//!
//! ...and type checking!
//! ```text
//! > add 1 world
//! Error: failed to parse argument value 'world': invalid digit found in string
//! Usage: add X:i32 Y:i32
//! ```
//!
//! It includes automatic `help` and `quit` commands. The help message is auto-generated:
//! ```text
//! > help
//! Available commands:
//!   add X:i32 Y:i32    Add X to Y
//!   hello name:String  Say hello
//!
//! Other commands:
//!   help  Show this help message
//!   quit  Quit repl
//! ```
//!
//! By default user does not have to use full command names, if the command name can be
//! resloved unambigiously (i.e. prefix matches only a single command), e.g.
//! ```text
//! > a 1 2
//! 1 + 2 = 3
//! ```
//! but if the input is ambigious, an error will be printed with command suggestions:
//! ```text
//! > h world
//! Command not found: h
//! Candidates:
//!   hello
//!   help
//! Use 'help' to see available commands.
//! ```
//!
//! The REPL also by default automatically implements command hints and TAB-completion (see [`rustyline::hint`], [`rustyline::completion`]).

pub mod command;
mod completion;
pub mod repl;

pub use anyhow;

pub use command::{Command, CommandStatus, Critical, CriticalError};
pub use repl::Repl;
