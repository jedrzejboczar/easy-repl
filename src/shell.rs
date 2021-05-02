use std::collections::HashMap;

use rustyline;
use rustyline::error::ReadlineError;

use crate::command::{Command, CommandStatus, ArgsError};

pub struct Shell<'a> {
    description: String,
    prompt: String,
    commands: HashMap<String, Command<'a>>,
    editor: rustyline::Editor<()>,
}


impl<'a> Shell<'a> {
    pub const RESERVED: &'static [(&'static str, &'static str)] = &[
        ("help", "Show this help message"),
        ("quit", "Quit shell"),
    ];

    pub fn new(prompt: &str, description: &str) -> Self {
        Self {
            description: description.into(),
            prompt: prompt.into(),
            commands: HashMap::new(),
            editor: rustyline::Editor::<()>::new()
        }
    }

    pub fn add(&mut self, name: &str, cmd: Command<'a>) {
        if Self::RESERVED.iter().find(|&&e| e.0 == name).is_some() {
            eprintln!("Command {} is a reserved name and will be ignored!", name);
        }
        self.commands.insert(name.into(), cmd);
    }

    pub fn help(&self) -> String {

        // let width = [width, Self::RESERVED.iter().map(|(name, desc)| name).max_by_key(|name| name.len())

        let format_entries = |entries: &[(String, String)]| {
            let width = entries.iter().map(|(sig, _)| sig).max_by_key(|sig| sig.len()).unwrap().len();
            entries.iter()
                .map(|(sig, desc)| format!("  {:width$}  {}", sig, desc, width = width))
                .collect::<Vec<_>>().join("\n")
        };

        // sort names
        let mut names: Vec<_> = self.commands.keys().collect();
        names.sort();

        let signature = |name: &String| format!("{} {}", name, self.commands[name].args_info.join(" "));
        let user: Vec<_> = names.iter()
            .map(|name| (signature(name), self.commands[name.as_str()].description.clone()))
            .collect();

        let other: Vec<_> = Self::RESERVED.iter().map(|(name, desc)| (name.to_string(), desc.to_string())).collect();

        let msg = format!(r#"
{}

Available commands:
{}

Other commands:
{}
        "#, self.description, format_entries(&user), format_entries(&other));
        msg.trim().into()
    }

    pub fn next(&mut self) -> bool {
        match self.editor.readline(&self.prompt) {
            Ok(line) => {
                let args: Vec<&str> = line.trim().split(char::is_whitespace).collect();
                if args.len() != 0 && args[0].len() > 0 {
                    match self.find_command(args[0]) {
                        None => {
                            println!("Command not found: {}", args[0]);
                        },
                        Some(cmd_name) => {
                            // handle errors/return
                            match self.handle_command(&cmd_name, &args[1..]) {
                                Ok(CommandStatus::Done) => {},
                                Ok(CommandStatus::Quit) => {
                                    return false;
                                },
                                Ok(CommandStatus::Failure(err)) => {
                                    println!("Command failed: {}", err);
                                },
                                Err(err) => {
                                    // in case of ArgsError it cannot have been reserved command
                                    let cmd = self.commands.get_mut(&cmd_name).unwrap();
                                    println!("Error: {}", err);
                                    println!("Usage: {}", cmd.args_info.join(" "))
                                }
                            };
                        }
                    }
                    self.editor.add_history_entry(line);
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                return false;
            },
            Err(ReadlineError::Eof) => {
                // println!("CTRL-D");
                return false;
            },
            Err(err) => {
                println!("Error: {:?}", err);
            },
        };
        true
    }

    fn find_command(&self, name: &str) -> Option<String> {
        if Self::RESERVED.iter().find(|&&n| n.0 == name).is_some() {
            Some(name.into())
        } else if self.commands.contains_key(name) {
            Some(name.into())
        } else {
            None
        }
    }

    fn handle_command(&mut self, name: &str, args: &[&str]) -> Result<CommandStatus, ArgsError> {
        match name {
            "help" => {
                println!("{}", self.help());
                Ok(CommandStatus::Done)
            },
            "quit" => Ok(CommandStatus::Quit),
            _ => {
                let cmd = self.commands.get_mut(name).unwrap();  // find_command must have returned correct name
                cmd.run(args)
            }
        }
    }

    pub fn run(&mut self) {
        while self.next() {}
    }
}
