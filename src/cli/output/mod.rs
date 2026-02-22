//! CLI output sinks and the high-level UI renderer.

pub trait Output {
    /// Whether sink is interactive/TTY.
    fn is_tty(&self) -> bool;

    /// Write a line to stdout.
    fn println(&self, line: &str);

    /// Write a line to stderr.
    fn eprintln(&self, line: &str);
}

mod terminal;
pub use terminal::Term;

mod ui;
pub use ui::Ui;

#[cfg(test)]
mod test;

#[cfg(test)]
pub use test::Test;
