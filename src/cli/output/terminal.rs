use std::io::IsTerminal;

use super::Output;

#[derive(Debug, Default, Clone, Copy)]
pub struct Term;

impl Output for Term {
    fn is_tty(&self) -> bool {
        std::io::stderr().is_terminal()
    }

    fn println(&self, line: &str) {
        println!("{line}");
    }

    fn eprintln(&self, line: &str) {
        eprintln!("{line}");
    }
}
