extern crate instru;

use instru::*;

fn main() {
    let x = Wrapper::new(Class::Fn, "fnname", "modpath::path",  "examples/main.rs", 1);
    let y = Wrapper::new(Class::Fn, "fnname", "modpath::path",  "examples/main.rs", 2);
    let z = Wrapper::new(Class::Fn, "fnname", "modpath::path",  "examples/main.rs", 3);
    let o = Wrapper::new(Class::Fn, "otherfn", "modpath::path", "examples/main.rs", 4);
}
