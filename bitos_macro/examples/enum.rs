use bitos::integer::*;
use bitos_macro::bitos;

#[bitos(2)]
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    A,
    B,
    C,
}

fn main() {}
