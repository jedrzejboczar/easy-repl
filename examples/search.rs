// use radix_trie::{Trie, TrieCommon};
use cedarwood::Cedar;
use trie_rs::TrieBuilder;

fn main() {
    let commands = [
        "help",
        "hard",
        "hell",
        "hardware",
        "quit",
        "query",
        "exit",
    ];
    let inputs = [
        "help",
        "he",
        "h",
        "q",
        "e",
    ];


    // Seems like we cannot search strings by common prefix
    //
    // let mut t = Trie::new();
    // for (i, cmd) in commands.iter().enumerate() {
    //     t.insert(cmd, i);
    // }
    //
    // for input in inputs.iter() {
    //     println!("Input = {}", input);
    //     for child in t.get_ancestor(input).unwrap().children() {
    //         println!("  {:?} => {:?}", child.key(), child.value());
    //     }
    // }

    println!("\n# cedarwood");
    let mut cedar = Cedar::new();
    let entries: Vec<_> = commands.iter().enumerate().map(|(i, c)| (*c, i as i32)).collect();
    cedar.build(&entries);
    for input in inputs.iter() {
        println!("Input = {}", input);
        for (i, u) in cedar.common_prefix_predict(input).unwrap().iter() {
            // println!("  {:?} => {:?}", i, u);
            println!("  ({}, {}), ({}, {})", i, u, entries[*i as usize].0, entries[*i as usize].1);
        }
    }
    // for (j, (c, i)) in entries.iter().enumerate() {
    //     println!("{} ({} {})", j, c, i)
    // }

    println!("\n# trie_rs");
    let mut builder = TrieBuilder::new();
    for c in commands.iter() {
        builder.push(*c);
    }
    let trie = builder.build();
    for input in inputs.iter() {
        println!("Input = {}", input);
        let results_u8 = trie.predictive_search(input);
        let results_str: Vec<_> = results_u8.iter().map(|s| std::str::from_utf8(s).unwrap()).collect();
        for c in results_str.iter() {
            println!("  {:?}", c);
        }
    }

}
