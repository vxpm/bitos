#![warn(clippy::semicolon_if_nothing_returned)]

use bitos::{integer::u7, prelude::*};

#[bitos(8)]
#[derive(Debug)]
pub struct Person {
    #[bits(1..7)]
    age: u7,
    #[bits(7)]
    alive: bool,
}

fn main() {
    // let person = Person::<i7>::from_bits(0b100_0000);
    // assert_eq!(person.age().value(), 0b1100_0000u8 as i8)
}
