use bitos::integer::*;
use bitos_macro::bitos;

#[bitos(2)]
#[derive(Debug)]
pub enum Kind {
    A,
    B,
    C,
}

fn main() {
    // let person = Person::<i7>::from_bits(0b100_0000);
    // assert_eq!(person.age().value(), 0b1100_0000u8 as i8)
}
