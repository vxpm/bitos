#![warn(clippy::semicolon_if_nothing_returned)]

use bitos::{Bits, integer::*};
use bitos_macro::bitos;

#[bitos(8)]
#[derive(Debug)]
pub struct Person<T>
where
    T: Bits,
{
    #[bits(0..7)]
    age: T,
    #[bits(7)]
    alive: bool,
}

// #[derive(Debug)]
// #[bitos(40)]
// pub struct FriendGroup {
//     #[bits(0..32)]
//     members: [Person<u8>; 4],
//     #[bits(32..)]
//     favorite_number: u8,
// }

fn main() {
    // let person = Person::<i7>::from_bits(0b100_0000);
    // assert_eq!(person.age().value(), 0b1100_0000u8 as i8)
}
