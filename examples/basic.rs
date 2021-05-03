use easy_repl::{Shell, CommandStatus, command, args_validator};
use anyhow::{self, Context};

fn main() -> anyhow::Result<()> {
    let mut outside_x = String::from("Out x");
    let mut outside_y = String::from("Out y");

    let mut shell = Shell::builder()
        .description("Example shell")
        .prompt("=> ")
        .add("count", command! {
            "Count from X to Y",
            i32:X i32:Y => |(x, y)| {
                for i in x..=y {
                    print!(" {}", i);
                }
                println!();
                CommandStatus::Done
            }
        })
        .add("say", command! {
            "Say X",
            f32 => |(x, )| {
                println!("x is equal to {}", x);
                CommandStatus::Done
            },
        })
        .add("outx", command! {
            "Use mutably outside var x. This command has a really long description so we need to wrap it somehow, it is interesting how actually the wrapping will be performed.",
            => |()| {
                outside_x += "x";
                println!("{}", outside_x);
                CommandStatus::Done
            },
        })
        .add("outy", easy_repl::Command {
            description: "Use mutably outside var y".into(),
            args_info: vec![],
            handler: Box::new(|_args| {
                outside_y += "y";
                println!("{}", outside_y);
                CommandStatus::Done
            }),
            validator: Box::new(args_validator!()),
        })
        .build().context("Failed to create shell")?;

    shell.run();

    Ok(())
}
