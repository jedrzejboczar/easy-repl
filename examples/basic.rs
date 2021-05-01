use std::collections::HashMap;

// TODO: merge all of these into a single macro
use just_repl::{Shell, CommandStatus, command, args_validator, replace_expr};

fn main() {
    let mut outside_x = String::from("Out x");
    let mut outside_y = String::from("Out y");

    let mut shell = Shell::new("=> ", "Example shell");

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

    shell.add("outx", command! {
        "Use mutably outside var x",
        => |()| {
            outside_x += "x";
            println!("{}", outside_x);
            CommandStatus::Done
        },
    });

    shell.add("outy", just_repl::Command {
        description: "Use mutably outside var y".into(),
        args_info: vec![],
        handler: Box::new(|args| {
            outside_y += "y";
            println!("{}", outside_y);
            CommandStatus::Done
        }),
        validator: Box::new(args_validator!()),
    });

    shell.run();
}
