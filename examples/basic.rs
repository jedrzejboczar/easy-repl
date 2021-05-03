use easy_repl::{Repl, CommandStatus, command, args_validator};
use anyhow::{self, Context};

fn main() -> anyhow::Result<()> {
    let mut outside_x = String::from("Out x");
    let mut outside_y = String::from("Out y");

    let mut repl = Repl::builder()
        .description("Example repl")
        .prompt("=> ")
        .add("count", command! {
            "Count from X to Y",
            i32:X i32:Y => |(x, y)| {
                for i in x..=y {
                    print!(" {}", i);
                }
                println!();
                Ok(CommandStatus::Done)
            }
        })
        .add("say", command! {
            "Say X",
            f32 => |(x, )| {
                println!("x is equal to {}", x);
                Ok(CommandStatus::Done)
            },
        })
        .add("outx", command! {
            "Use mutably outside var x. This command has a really long description so we need to wrap it somehow, it is interesting how actually the wrapping will be performed.",
            => |()| {
                outside_x += "x";
                println!("{}", outside_x);
                Ok(CommandStatus::Done)
            },
        })
        .add("outy", easy_repl::Command {
            description: "Use mutably outside var y".into(),
            args_info: vec![],
            handler: Box::new(|_args| {
                outside_y += "y";
                println!("{}", outside_y);
                Ok(CommandStatus::Done)
            }),
            validator: Box::new(args_validator!()),
        })
        .build().context("Failed to create repl")?;

    repl.run().context("Critical REPL error");

    Ok(())
}
