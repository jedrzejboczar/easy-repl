use std::collections::HashMap;

use rustyline;
use rustyline::error::ReadlineError;

use crate::command::{Command, CommandStatus, ArgsError};

pub struct Shell {
    pub description: String,
    pub prompt: String,
    pub commands: HashMap<String, Command>,
    pub editor: rustyline::Editor<()>,
}

impl Shell {
    pub fn add(&mut self, description: &str, cmd: Command) {
        self.commands.insert(description.into(), cmd);
    }

    pub fn help(&self) -> String {
        // sort names
        let mut names: Vec<_> = self.commands.keys().collect();
        names.sort();
        // find width for the alignment
        let signature = |name: &String| format!("  {} {}", name, self.commands[name].args_info.join(" "));
        let width = names.iter().cloned().map(signature).max_by_key(|sig| sig.len()).unwrap().len();
        let signatures: Vec<_> = names.iter()
            .map(|name| format!("{:width$}  {}",
                signature(name), self.commands[*name].description, width = width))
            .collect();
        let msg = format!(r#"
{}

Available commands:
{}
        "#, self.description, signatures.join("\n"));
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
                            // find_command must have returned correct name
                            let cmd = self.commands.get_mut(&cmd_name).unwrap();
                            // handle errors/return
                            match cmd.run(&args[1..]) {
                                Ok(CommandStatus::Done) => {},
                                Ok(CommandStatus::Quit) => {
                                    return false;
                                },
                                Ok(CommandStatus::Failure(err)) => {
                                    println!("Command failed: {}", err);
                                },
                                Err(err) => {
                                    println!("Error: {}", err);
                                    println!("Usage: {}", cmd.args_info.join(" "))
                                }
                            };
                            self.editor.add_history_entry(line);
                        }
                    }
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
        if self.commands.contains_key(name) {
            Some(name.into())
        } else {
            None
        }
    }

    fn handle_command(&mut self, name: String, args: &[&str]) -> Result<CommandStatus, ArgsError> {
        let cmd = self.commands.get_mut(&name).unwrap();  // find_command must have returned correct name
        cmd.run(args)
    }

    pub fn run(&mut self) {
        use crate::{command, args_validator};
        self.add("help", command! {
            "Show help",
            => |()| {
                println!("{}", self.help());
                CommandStatus::Done
            },
        });

        while self.next() {}
    }
}
