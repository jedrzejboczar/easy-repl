use std::{collections::HashMap, io::Write, rc::Rc};

use rustyline::{self, completion::FilenameCompleter, error::ReadlineError};
use textwrap;
use thiserror;
use trie_rs::{Trie, TrieBuilder};

use crate::command::{Command, CommandStatus, ArgsError};
use crate::completion::{Completion, completion_candidates};

pub const RESERVED: &'static [(&'static str, &'static str)] = &[
    ("help", "Show this help message"),
    ("quit", "Quit shell"),
];

pub struct Shell<'a> {
    description: String,
    prompt: String,
    text_width: usize,
    commands: HashMap<String, Command<'a>>,
    trie: Rc<Trie<u8>>,
    editor: rustyline::Editor<Completion>,
    out: Box<dyn Write>,
    predict_commands: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoopStatus {
    Continue,
    Break,
}

pub struct ShellBuilder<'a> {
    commands: Vec<(String, Command<'a>)>,
    description: String,
    prompt: String,
    text_width: usize,
    editor_config: rustyline::config::Config,
    out: Box<dyn Write>,
    with_hints: bool,
    with_completion: bool,
    with_filename_completion: bool,
    predict_commands: bool,
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

pub(crate) fn split_args(line: &str) -> Vec<&str> {
    line.trim().split(char::is_whitespace).collect()
}

impl<'a> Default for ShellBuilder<'a> {
    fn default() -> Self {
        ShellBuilder {
            prompt: "> ".into(),
            text_width: 80,
            description: Default::default(),
            commands: Default::default(),
            out: Box::new(std::io::stderr()),
            editor_config: rustyline::config::Config::builder()
                .output_stream(rustyline::OutputStreamType::Stderr)  // NOTE: cannot specify `out`
                .completion_type(rustyline::CompletionType::List)
                .build(),
            with_hints: true,
            with_completion: true,
            with_filename_completion: false,
            predict_commands: true,
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
    setter!(editor_config: rustyline::config::Config);
    setter!(out: Box<dyn Write>);
    setter!(with_hints: bool);
    setter!(with_completion: bool);
    setter!(with_filename_completion: bool);
    setter!(predict_commands: bool);

    pub fn add(mut self, name: &str, cmd: Command<'a>) -> Self {
        self.commands.push((name.into(), cmd));
        self
    }
    pub fn build(self) -> Result<Shell<'a>, ShellBuilderError> {
        let mut commands = HashMap::new();
        let mut trie = TrieBuilder::new();
        for (name, cmd) in self.commands.into_iter() {
            let old = commands.insert(name.clone(), cmd);
            if split_args(&name).len() != 1 || name.is_empty() {
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

        let trie = Rc::new(trie.build());
        let helper = Completion {
            trie: trie.clone(),
            with_hints: self.with_hints,
            with_completion: self.with_completion,
            filename_completer: if self.with_filename_completion {
                Some(FilenameCompleter::new())
            } else {
                None
            },
        };
        let mut editor = rustyline::Editor::with_config(rustyline::config::Config::builder()
            .output_stream(rustyline::OutputStreamType::Stderr)  // NOTE: cannot specify `out`
            .completion_type(rustyline::CompletionType::List)
            .build());
        editor.set_helper(Some(helper));

        Ok(Shell {
            description: self.description,
            prompt: self.prompt,
            text_width: self.text_width,
            commands,
            trie,
            editor,
            out: self.out,
            predict_commands: self.predict_commands,
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
        let args: Vec<&str> = split_args(&line);
        let prefix = args[0];
        let mut candidates = completion_candidates(&self.trie, prefix);
        let exact = candidates.len() == 1 && candidates[0] == prefix;
        if candidates.len() != 1 || (!self.predict_commands && !exact) {
            writeln!(&mut self.out, "Command not found: {}", prefix).unwrap();
            if candidates.len() > 1 || (!self.predict_commands && !exact) {
                candidates.sort();
                writeln!(&mut self.out, "Candidates:\n  {}", candidates.join("\n  ")).unwrap();
            }
            writeln!(&mut self.out, "Use 'help' to see available commands.").unwrap();
            LoopStatus::Continue
        } else {
            let name = &candidates[0];
            match self.handle_command(name, &args[1..]) {
                Ok(CommandStatus::Done) => LoopStatus::Continue,
                Ok(CommandStatus::Quit) => LoopStatus::Break,
                Ok(CommandStatus::Failure(err)) => {
                    writeln!(&mut self.out, "Command failed: {}", err).unwrap();
                    LoopStatus::Continue
                },
                Err(err) => {
                    // in case of ArgsError it cannot have been reserved command
                    let cmd = self.commands.get_mut(name).unwrap();
                    writeln!(&mut self.out, "Error: {}", err).unwrap();
                    writeln!(&mut self.out, "Usage: {}", cmd.args_info.join(" ")).unwrap();
                    LoopStatus::Continue
                }
            }
        }
    }

    pub fn next(&mut self) -> LoopStatus {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                if !line.trim().is_empty() {
                    self.editor.add_history_entry(line.trim());
                    self.handle_line(line)
                } else {
                    LoopStatus::Continue
                }
            },
            Err(ReadlineError::Interrupted) => {
                writeln!(&mut self.out, "CTRL-C").unwrap();
                LoopStatus::Break
            },
            Err(ReadlineError::Eof) => {
                LoopStatus::Break
            },
            Err(err) => {
                writeln!(&mut self.out, "Error: {:?}", err).unwrap();
                LoopStatus::Continue
            },
        }
    }

    fn handle_command(&mut self, name: &str, args: &[&str]) -> Result<CommandStatus, ArgsError> {
        match name {
            "help" => {
                let help = self.help();
                writeln!(&mut self.out, "{}", help).unwrap();
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
