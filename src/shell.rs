use std::collections::HashMap;

use rustyline::{self, error::ReadlineError};
use textwrap;
use thiserror;
use trie_rs::{Trie, TrieBuilder};

use crate::command::{Command, CommandStatus, ArgsError};

pub const RESERVED: &'static [(&'static str, &'static str)] = &[
    ("help", "Show this help message"),
    ("quit", "Quit shell"),
];

pub struct Shell<'a> {
    description: String,
    prompt: String,
    text_width: usize,
    commands: HashMap<String, Command<'a>>,
    trie: Trie<u8>,
    editor: rustyline::Editor<()>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoopStatus {
    Continue,
    Break,
}

#[derive(Debug)]
pub struct ShellBuilder<'a> {
    description: String,
    prompt: String,
    text_width: usize,
    commands: Vec<(String, Command<'a>)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ShellBuilderError {
    #[error("more than one command with name '{0}' added")]
    DuplicateCommands(String),
    #[error("name '{0}' contains spaces or is empty, thus would be impossible to call")]
    NameWithSpaces(String),
    #[error("'{0}' is a reserved command name")]
    ReservedName(String),
}

impl<'a> Default for ShellBuilder<'a> {
    fn default() -> Self {
        ShellBuilder {
            prompt: "> ".into(),
            text_width: 80,
            description: Default::default(),
            commands: Default::default(),
        }
    }
}

macro_rules! setter {
    ($name:ident: $type:ty) => {
        pub fn $name<T: Into<$type>>(mut self, v: T) -> Self {
            self.$name = v.into();
            self
        }
    };
}

impl<'a> ShellBuilder<'a> {
    setter!(description: String);
    setter!(prompt: String);
    setter!(text_width: usize);

    pub fn add(mut self, name: &str, cmd: Command<'a>) -> Self {
        self.commands.push((name.into(), cmd));
        self
    }
    pub fn build(self) -> Result<Shell<'a>, ShellBuilderError> {
        let mut commands = HashMap::new();
        let mut trie = TrieBuilder::new();
        for (name, cmd) in self.commands.into_iter() {
            let old = commands.insert(name.clone(), cmd);
            if name.split(char::is_whitespace).count() != 1 || name.is_empty() {
                return Err(ShellBuilderError::NameWithSpaces(name));
            } else if RESERVED.iter().find(|&&(n, _)| n == name).is_some() {
                return Err(ShellBuilderError::ReservedName(name));
            } else if old.is_some() {
                return Err(ShellBuilderError::DuplicateCommands(name));
            }
            trie.push(name);
        }
        for (name, _) in RESERVED.iter() {
            trie.push(name);
        }
        Ok(Shell {
            description: self.description,
            prompt: self.prompt,
            text_width: self.text_width,
            commands,
            trie: trie.build(),
            editor: rustyline::Editor::<()>::new(),
        })
    }
}

impl<'a> Shell<'a> {
    pub fn builder() -> ShellBuilder<'a> {
        ShellBuilder::default()
    }

    fn format_help_entries(&self, entries: &[(String, String)]) -> String {
        if entries.is_empty() {
            return "".into();
        }
        let width = entries.iter()
            .map(|(sig, _)| sig)
            .max_by_key(|sig| sig.len())
            .unwrap().len();
        entries.iter()
            .map(|(sig, desc)| {
                let indent = " ".repeat(width + 2 + 2);
                let opts = textwrap::Options::new(self.text_width)
                    .initial_indent("")
                    .subsequent_indent(&indent);
                let line = format!("  {:width$}  {}", sig, desc, width = width);
                textwrap::fill(&line, &opts)
            })
            .reduce(|mut out, next| {
                out.push_str("\n");
                out.push_str(&next);
                out
            }).unwrap()
    }

    pub fn help(&self) -> String {
        let mut names: Vec<_> = self.commands.keys().collect();
        names.sort();

        let signature = |name: &String| format!("{} {}", name, self.commands[name].args_info.join(" "));
        let user: Vec<_> = names.iter()
            .map(|name| (signature(name), self.commands[name.as_str()].description.clone()))
            .collect();

        let other: Vec<_> = RESERVED.iter().map(|(name, desc)| (name.to_string(), desc.to_string())).collect();

        let msg = format!(r#"
{}

Available commands:
{}

Other commands:
{}
        "#, self.description, self.format_help_entries(&user), self.format_help_entries(&other));
        msg.trim().into()
    }

    fn handle_line(&mut self, line: String) -> LoopStatus {
        // line must not be empty
        let args: Vec<&str> = line.trim().split(char::is_whitespace).collect();
        let prefix = args[0];
        let mut candidates = self.find_command(prefix);
        if candidates.len() != 1 {
            eprintln!("Command not found: {}", prefix);
            if candidates.len() > 1 {
                candidates.sort();
                eprintln!("Candidates:\n  {}", candidates.join("\n  "));
            }
            eprintln!("Use 'help' to see available commands.");
            LoopStatus::Continue
        } else {
            let name = &candidates[0];
            match self.handle_command(name, &args[1..]) {
                Ok(CommandStatus::Done) => LoopStatus::Continue,
                Ok(CommandStatus::Quit) => LoopStatus::Break,
                Ok(CommandStatus::Failure(err)) => {
                    eprintln!("Command failed: {}", err);
                    LoopStatus::Continue
                },
                Err(err) => {
                    // in case of ArgsError it cannot have been reserved command
                    let cmd = self.commands.get_mut(name).unwrap();
                    eprintln!("Error: {}", err);
                    eprintln!("Usage: {}", cmd.args_info.join(" "));
                    LoopStatus::Continue
                }
            }
        }
    }

    pub fn next(&mut self) -> LoopStatus {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                if !line.trim().is_empty() {
                    self.editor.add_history_entry(line.clone());
                    self.handle_line(line)
                } else {
                    LoopStatus::Continue
                }
            },
            Err(ReadlineError::Interrupted) => {
                eprintln!("CTRL-C");
                LoopStatus::Break
            },
            Err(ReadlineError::Eof) => {
                // println!("CTRL-D");
                LoopStatus::Break
            },
            Err(err) => {
                eprintln!("Error: {:?}", err);
                LoopStatus::Continue
            },
        }
    }

    fn find_command(&self, name: &str) -> Vec<String> {
        self.trie.predictive_search(name).into_iter()
            .map(|bytes| String::from_utf8(bytes).unwrap())
            .collect()
    }

    fn handle_command(&mut self, name: &str, args: &[&str]) -> Result<CommandStatus, ArgsError> {
        match name {
            "help" => {
                println!("{}", self.help());
                Ok(CommandStatus::Done)
            },
            "quit" => Ok(CommandStatus::Quit),
            _ => {
                // find_command must have returned correct name
                let cmd = self.commands.get_mut(name).unwrap();
                cmd.run(args)
            }
        }
    }

    pub fn run(&mut self) {
        while let LoopStatus::Continue = self.next() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command;

    #[test]
    fn builder_duplicate() {
        let result = Shell::builder()
            .add("name_x", command!("", => |()| CommandStatus::Done))
            .add("name_x", command!("", => |()| CommandStatus::Done))
            .build();
        assert!(matches!(result, Err(ShellBuilderError::DuplicateCommands(_))));
    }

    #[test]
    fn builder_empty() {
        let result = Shell::builder()
            .add("", command!("", => |()| CommandStatus::Done))
            .build();
        assert!(matches!(result, Err(ShellBuilderError::NameWithSpaces(_))));
    }

    #[test]
    fn builder_spaces() {
        let result = Shell::builder()
            .add("name-with spaces", command!("", => |()| CommandStatus::Done))
            .build();
        assert!(matches!(result, Err(ShellBuilderError::NameWithSpaces(_))));
    }

    #[test]
    fn builder_reserved() {
        let result = Shell::builder()
            .add("help", command!("", => |()| CommandStatus::Done))
            .build();
        assert!(matches!(result, Err(ShellBuilderError::ReservedName(_))));
        let result = Shell::builder()
            .add("quit", command!("", => |()| CommandStatus::Done))
            .build();
        assert!(matches!(result, Err(ShellBuilderError::ReservedName(_))));
    }

    #[test]
    fn shell_quits() {
        let mut shell = Shell::builder()
            .add("foo", command!("description", => |()| CommandStatus::Done))
            .build().unwrap();
        assert_eq!(shell.handle_line("quit".into()), LoopStatus::Break);
        let mut shell = Shell::builder()
            .add("foo", command!("description", => |()| CommandStatus::Quit))
            .build().unwrap();
        assert_eq!(shell.handle_line("foo".into()), LoopStatus::Break);
    }
}
