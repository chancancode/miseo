use std::{cell::RefCell, rc::Rc};

use super::Output;

/// Test output sink that records stdout/stderr lines in memory.
#[derive(Debug, Clone)]
pub struct Test {
    tty: bool,
    out: Rc<RefCell<Vec<String>>>,
    err: Rc<RefCell<Vec<String>>>,
}

impl Test {
    pub fn new(tty: bool) -> Self {
        Self {
            tty,
            out: Rc::new(RefCell::new(vec![])),
            err: Rc::new(RefCell::new(vec![])),
        }
    }

    fn out_lines(&self) -> Vec<String> {
        self.out.borrow().clone()
    }

    fn err_lines(&self) -> Vec<String> {
        self.err.borrow().clone()
    }

    /// Clear captured stdout/stderr lines.
    pub fn clear(&self) {
        self.out.borrow_mut().clear();
        self.err.borrow_mut().clear();
    }

    /// Assert stdout lines match `expected`.
    ///
    /// Pattern syntax: `...` is a wildcard matching any substring.
    pub fn assert_out(&self, expected: &[&str]) {
        assert_lines(self.out_lines(), expected);
    }

    /// Assert stderr lines match `expected`.
    ///
    /// Pattern syntax: `...` is a wildcard matching any substring.
    pub fn assert_err(&self, expected: &[&str]) {
        assert_lines(self.err_lines(), expected);
    }
}

impl Output for Test {
    fn is_tty(&self) -> bool {
        self.tty
    }

    fn println(&self, line: &str) {
        self.out.borrow_mut().push(line.to_string());
    }

    fn eprintln(&self, line: &str) {
        self.err.borrow_mut().push(line.to_string());
    }
}

fn line_matches(line: &str, pattern: &str) -> bool {
    if !pattern.contains("...") {
        return line == pattern;
    }

    let starts_wild = pattern.starts_with("...");
    let ends_wild = pattern.ends_with("...");
    let parts: Vec<&str> = pattern
        .split("...")
        .filter(|part| !part.is_empty())
        .collect();

    if parts.is_empty() {
        return true;
    }

    let mut rest = line;
    for (index, part) in parts.iter().enumerate() {
        if index == 0 && !starts_wild {
            let Some(next) = rest.strip_prefix(part) else {
                return false;
            };
            rest = next;
            continue;
        }

        let Some(found) = rest.find(part) else {
            return false;
        };
        rest = &rest[found + part.len()..];
    }

    ends_wild || rest.is_empty()
}

fn assert_lines(lines: Vec<String>, expected: &[&str]) {
    assert_eq!(lines.len(), expected.len());

    for (index, (line, pattern)) in lines.iter().zip(expected).enumerate() {
        assert!(
            line_matches(line, pattern),
            "line {index} did not match\n  pattern: {pattern:?}\n   actual: {line:?}"
        );
    }
}
