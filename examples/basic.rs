use std::collections::HashMap;

// TODO: merge all of these into a single macro
use just_repl::{Shell, CommandStatus, command, args_validator, replace_expr};

fn main() {
    let mut shell = Shell {
        description: "Example shell".into(),
        prompt: "=> ".into(),
        commands: HashMap::new(),
        editor: rustyline::Editor::<()>::new(),
    };

    // shell.add("help", Command {
    //     description: "Print help".into(),
    //     handler: Box::new(|args| {
    //         println!("This is help message");
    //         0
    //     }),
    //     parser: typed_parser!(),
    // });
    //
    // shell.add("hello", Command {
    //     description: "Say hello".into(),
    //     handler: Box::new(|args| {
    //         println!("Hello {}!", args[0]);
    //         0
    //     }),
    //     parser: typed_parser!(String),
    // });
    //
    // shell.add("hello", command! {
    //     "Say hello",
    //     String="who" => |(who, )| {
    //         println!("Hello {}!", who);
    //         0
    //     }
    // });


    shell.add("count", command! {
        "Count from X to Y",
        i32:X i32:Y => |(x, y)| {
            for i in x..=y {
                print!(" {}", i);
            }
            println!();
            CommandStatus::Done
        }
    });

    shell.add("say", command! {
        "Say X",
        f32 => |(x, )| {
            println!("x is equal to {}", x);
            CommandStatus::Done
        },
    });

    let msg = shell.help();

    // shell.add("help", command! {
    //     "Show help",
    //     => |()| {
    //         println!("{}", msg);
    //         CommandStatus::Done
    //     },
    // });

    shell.run();
}
