use std::rc::Rc;

use rustyline::{completion::{Completer, FilenameCompleter, Pair}, hint::Hinter};
use rustyline_derive::{Helper, Highlighter, Validator};
use trie_rs::{Trie, TrieBuilder};

use crate::shell::split_args;

#[derive(Helper, Validator, Highlighter)]
pub(crate) struct Completion {
    pub(crate) trie: Rc<Trie<u8>>,
    pub(crate) with_hints: bool,
    pub(crate) with_completion: bool,
    pub(crate) filename_completer: Option<FilenameCompleter>,
}

impl Hinter for Completion {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        if !self.with_hints {
            return None;
        }
        let prefix = &line[..pos];
        if pos < line.len() || prefix.is_empty() {
            None
        } else {
            let candidates = completion_candidates(&self.trie, prefix);
            if candidates.len() == 1 {
                Some(candidates[0][pos..].into())
            } else {
                None
            }
        }
    }
}

impl Completer for Completion {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        if !self.with_completion {
            return Ok((0, Vec::with_capacity(0)));
        }
        // TODO: revise this logic when we actually start using filename completer
        if let Some(completion) = self.complete_command(line, pos, ctx)? {
            Ok(completion)
        } else if let Some(completer) = self.filename_completer.as_ref() {
            completer.complete(line, pos, ctx)
        } else {
            Ok((0, Vec::with_capacity(0)))
        }
    }
}

impl Completion {
    fn complete_command(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<Option<(usize, Vec<<Self as Completer>::Candidate>)>> {
        let args = split_args(line);
        let on_first = args.is_empty() || pos == args[0].len();
        let completions = if on_first {
            let candidates = completion_candidates(&self.trie, args[0])
                .into_iter()
                .map(|c| Pair { display: c.clone(), replacement: c })
                .collect();
            Some((0, candidates))
        } else {
            None
        };
        Ok(completions)
    }
}

pub(crate) fn completion_candidates(trie: &Trie<u8>, prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        Vec::with_capacity(0)
    } else {
        trie.predictive_search(prefix).into_iter()
            .map(|bytes| String::from_utf8(bytes).unwrap())
            .collect()
    }
}

