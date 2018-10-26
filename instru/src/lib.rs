extern crate ansi_term;
extern crate difference;
extern crate smallvec;
extern crate lazy_static;

use ansi_term::*;
use difference::{Changeset, Difference};
use lazy_static::lazy_static;
use smallvec::SmallVec;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

struct ThreadState {
    indent: u32,
    prev_indent: u32,
    prev_str: Option<String>,
    sources: HashMap<String, Arc<Vec<String>>>,
}

lazy_static! {
    static ref FILES: Mutex<HashMap<String, Arc<Vec<String>>>> = Mutex::new(HashMap::new());
}

impl ThreadState {
    fn new() -> Self {
        Self {
            indent: 0,
            prev_indent: 0,
            prev_str: None,
            sources: HashMap::new(),
        }
    }

    fn get_line(&mut self, filename: &'static str, line_nr: u32) -> String {
        if line_nr == 0 {
            panic!("line number 0 is not valid");
        }

        if let Some(local_source) = self.sources.get(filename) {
            let lines = local_source.len();
            if line_nr as usize > lines {
                panic!("invalid line number {} for {}", line_nr, filename);
            }
            return local_source[line_nr as usize - 1].clone();
        }

        let mut files = FILES.lock().unwrap();
        let entry = files.entry(String::from(filename));

        let source = entry.or_insert_with(|| {
            let open_file = std::fs::File::open(filename).unwrap();
            let reader = std::io::BufReader::new(&open_file);
            use std::io::BufRead;
            let lines : Vec<_> = reader.lines().map(|x|x.unwrap()).collect();
            Arc::new(lines)
        });

        self.sources.insert(String::from(filename), source.clone());

        self.get_line(filename, line_nr)
    }
}

thread_local! {
    static STATE: UnsafeCell<ThreadState> = UnsafeCell::new(ThreadState::new());
}

#[derive(Clone, Copy)]
pub enum Class {
    Fn,
    Stmt,
    Block,
}

pub enum Pos {
    Enter,
    At,
    Leave,
}

pub struct Wrapper {
    class: Class,
    name: &'static str,
    modpath: &'static str,
    file: &'static str,
    line_nr: u32,
}

impl Wrapper {
    fn specifier(&self, pos: Pos) -> String {
        let line_nr = match pos {
            Pos::Enter => {
                format!(":(->{})", self.line_nr)
            }
            Pos::At => {
                format!(":(@@{})", self.line_nr)
            }
            Pos::Leave => {
                format!(":(<-{})", self.line_nr)
            }
        };
        match self.class {
            Class::Fn =>
                format!("{}:{}:{}{}",
                self.modpath, self.name,
                match pos {
                    Pos::Enter => {
                        "()"
                    }
                    Pos::At => {
                        ""
                    }
                    Pos::Leave => {
                        "<-"
                    }
                },
                line_nr),
            Class::Stmt =>
                format!("{}:{}:{}",
                self.modpath, self.name, line_nr),
            Class::Block =>
                format!("{}:{}:{}",
                self.modpath, self.name, line_nr),
        }
    }

    pub fn new(class: Class,
               name: &'static str,
               modpath: &'static str,
               file: &'static str,
               line_nr: u32) -> Self
    {
        let s = Self {
            name, modpath, file, line_nr, class
        };

        match class {
            Class::Stmt => {
                s.print(Pos::At);
            }
            Class::Fn | Class::Block => {
                s.print_enter();
                STATE.with(|state| {
                    let state = unsafe { &mut *state.get() };

                    state.indent += 1;
                });
            }
        }

        s
    }

    const MARGIN: usize = 50;

    fn print(&self, pos: Pos) {
        STATE.with(|state| {
            let state = unsafe { &mut *state.get() };
            let specifier = self.specifier(pos);
            let width = if specifier.len() <= Wrapper::MARGIN { Wrapper::MARGIN - specifier.len() } else {0};
            let prev_indent = state.prev_indent;
            state.prev_indent = state.indent;

            print!("[{}] ", {
                if prev_indent == state.indent {
                    Colour::RGB(222, 222, 222)
                } else if prev_indent < state.indent {
                    Colour::RGB(0, 222, 0)
                } else {
                    Colour::RGB(0, 122, 0)
                }
            } .paint(format!("{:2}", state.indent)));

            if let Some(prev_str) = &state.prev_str {
                let changeset = Changeset::new(prev_str, specifier.as_str(), "");
                let mut vec : SmallVec<[_; 0x20]> = SmallVec::new();
                let mut nr_changes = 0;
                for i in &changeset.diffs {
                    let fn_change = move |x| Colour::White.bold().paint(x);
                    let fn_same = move |x| Colour::RGB(120, 120, 120).paint(x);
                    match i {
                        Difference::Same(x) => {
                            if nr_changes > 0 {
                                vec.push(fn_change(x))
                            } else {
                                vec.push(fn_same(x))
                            }
                        }
                        Difference::Rem(_) => {
                            nr_changes += 1;
                        }
                        Difference::Add(x) => {
                            vec.push(fn_change(x));
                            nr_changes += 1;
                        }
                    }
                }
                print!("{}{:-width$}{}", ANSIStrings(vec.as_slice()), "", ":", width=width);
            } else {
                print!("{}{:-width$}{}", specifier, "", ":", width=width);
            }

            print!(" {}", state.get_line(self.file, self.line_nr));

            state.prev_str = Some(specifier);
        });
        println!();
    }

    fn print_enter(&self) {
        self.print(Pos::Enter);
    }

    fn print_leave(&self) {
        self.print(Pos::Leave);
    }
}

impl Drop for Wrapper {
    fn drop(&mut self) {
        match self.class {
            Class::Stmt => {}
            Class::Fn | Class::Block => {
                STATE.with(|state| {
                    let state = unsafe { &mut *state.get() };

                    state.indent -= 1;
                });
                self.print_leave();
            }
        }
    }
}

pub fn statement(
    name: &'static str,
    modpath: &'static str,
    file: &'static str,
    line_nr: u32)
{
    let _ = Wrapper::new(Class::Stmt, name, modpath, file, line_nr);
}
