use std::{cell::RefCell};
use easy_repl::{Repl, CommandStatus, command};
use anyhow::{self, Context};

fn main() -> anyhow::Result<()> {
    // To use a value in multiple commands we need shared ownership
    // combined with some kind of references.
    // This could be Rc<RefCell<_>>, but in this example it's possible
    // to avoid smart pointers and just use two references &RefCell<_>.
    let counter = RefCell::new(0);
    let ref1 = &counter;
    let ref2 = &counter;

    let mut repl = Repl::builder()
        .add("inc", command! {
            "Increment counter",
            () => || {
                *ref1.borrow_mut() += 1;
                println!("counter = {}", ref1.borrow());
                Ok(CommandStatus::Done)
            },
        })
        .add("dec", command! {
            "Decrement counter",
            () => || {
                *ref2.borrow_mut() -= 1;
                println!("counter = {}", ref2.borrow());
                Ok(CommandStatus::Done)
            },
        })
        .build().context("Failed to create repl")?;

    repl.run().context("Critical REPL error")?;

    Ok(())
}
