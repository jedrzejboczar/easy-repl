use std::{collections::HashMap, io::Write, rc::Rc};

use rustyline::{self, completion::FilenameCompleter, error::ReadlineError};
use textwrap;
use thiserror;
use trie_rs::{Trie, TrieBuilder};

use crate::command::{Command, CommandStatus, CriticalError, ArgsError};
use crate::completion::{Completion, completion_candidates};

pub const RESERVED: &'static [(&'static str, &'static str)] = &[
    ("help", "Show this help message"),
    ("quit", "Quit repl"),
];

pub struct Repl<'a> {
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

pub struct ReplBuilder<'a> {
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
pub enum BuilderError {
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

impl<'a> Default for ReplBuilder<'a> {
    fn default() -> Self {
        ReplBuilder {
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

impl<'a> ReplBuilder<'a> {
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
    pub fn build(self) -> Result<Repl<'a>, BuilderError> {
        let mut commands = HashMap::new();
        let mut trie = TrieBuilder::new();
        for (name, cmd) in self.commands.into_iter() {
            let old = commands.insert(name.clone(), cmd);
            if split_args(&name).len() != 1 || name.is_empty() {
                return Err(BuilderError::NameWithSpaces(name));
            } else if RESERVED.iter().find(|&&(n, _)| n == name).is_some() {
                return Err(BuilderError::ReservedName(name));
            } else if old.is_some() {
                return Err(BuilderError::DuplicateCommands(name));
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

        Ok(Repl {
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

impl<'a> Repl<'a> {
    pub fn builder() -> ReplBuilder<'a> {
        ReplBuilder::default()
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

    fn handle_line(&mut self, line: String) -> anyhow::Result<LoopStatus> {
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
            Ok(LoopStatus::Continue)
        } else {
            let name = &candidates[0];
            match self.handle_command(name, &args[1..]) {
                Ok(CommandStatus::Done) => Ok(LoopStatus::Continue),
                Ok(CommandStatus::Quit) => Ok(LoopStatus::Break),
                Err(err) if err.downcast_ref::<CriticalError>().is_some()  => {
                    Err(err)
                },
                Err(err) => {
                    // other errors are handler here
                    writeln!(&mut self.out, "Error: {}", err).unwrap();
                    if err.downcast_ref::<ArgsError>().is_some() {
                        // in case of ArgsError we know it could not have been a reserved command
                        let cmd = self.commands.get_mut(name).unwrap();
                        writeln!(&mut self.out, "Usage: {} {}", name, cmd.args_info.join(" ")).unwrap();
                    }
                    Ok(LoopStatus::Continue)
                }
            }
        }
    }

    pub fn next(&mut self) -> anyhow::Result<LoopStatus> {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                if !line.trim().is_empty() {
                    self.editor.add_history_entry(line.trim());
                    self.handle_line(line)
                } else {
                    Ok(LoopStatus::Continue)
                }
            },
            Err(ReadlineError::Interrupted) => {
                writeln!(&mut self.out, "CTRL-C").unwrap();
                Ok(LoopStatus::Break)
            },
            Err(ReadlineError::Eof) => {
                Ok(LoopStatus::Break)
            },
            // TODO: not sure if these should be propagated or handler here
            Err(err) => {
                writeln!(&mut self.out, "Error: {:?}", err).unwrap();
                Ok(LoopStatus::Continue)
            },
        }
    }

    fn handle_command(&mut self, name: &str, args: &[&str]) -> anyhow::Result<CommandStatus> {
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

    pub fn run(&mut self) -> anyhow::Result<()> {
        while let LoopStatus::Continue = self.next()? {}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command;

    #[test]
    fn builder_duplicate() {
        let result = Repl::builder()
            .add("name_x", command!("", => || Ok(CommandStatus::Done)))
            .add("name_x", command!("", => || Ok(CommandStatus::Done)))
            .build();
        assert!(matches!(result, Err(BuilderError::DuplicateCommands(_))));
    }

    #[test]
    fn builder_empty() {
        let result = Repl::builder()
            .add("", command!("", => || Ok(CommandStatus::Done)))
            .build();
        assert!(matches!(result, Err(BuilderError::NameWithSpaces(_))));
    }

    #[test]
    fn builder_spaces() {
        let result = Repl::builder()
            .add("name-with spaces", command!("", => || Ok(CommandStatus::Done)))
            .build();
        assert!(matches!(result, Err(BuilderError::NameWithSpaces(_))));
    }

    #[test]
    fn builder_reserved() {
        let result = Repl::builder()
            .add("help", command!("", => || Ok(CommandStatus::Done)))
            .build();
        assert!(matches!(result, Err(BuilderError::ReservedName(_))));
        let result = Repl::builder()
            .add("quit", command!("", => || Ok(CommandStatus::Done)))
            .build();
        assert!(matches!(result, Err(BuilderError::ReservedName(_))));
    }

    #[test]
    fn repl_quits() {
        let mut repl = Repl::builder()
            .add("foo", command!("description", => || Ok(CommandStatus::Done)))
            .build().unwrap();
        assert_eq!(repl.handle_line("quit".into()).unwrap(), LoopStatus::Break);
        let mut repl = Repl::builder()
            .add("foo", command!("description", => || Ok(CommandStatus::Quit)))
            .build().unwrap();
        assert_eq!(repl.handle_line("foo".into()).unwrap(), LoopStatus::Break);
    }
}
