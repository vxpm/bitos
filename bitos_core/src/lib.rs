pub mod integer;

use integer::{IsStorageForBits, SInt, UInt, UnsignedInt};

/// Trait for types that can try to be created from and turned into raw bits.
pub trait TryBits: Sized {
    /// The raw bits type.
    type Bits: UnsignedInt;

    /// Tries to create a value of this type from it's raw bit representation.
    fn try_from_bits(value: Self::Bits) -> Option<Self>;

    /// Turns this value into it's raw bit representation.
    fn into_bits(self) -> Self::Bits;
}

/// Trait for types that can be created from and turned into raw bits.
pub trait Bits: TryBits {
    /// Creates a value of this type from it's raw bit representation.
    fn from_bits(value: Self::Bits) -> Self;
}

macro_rules! impl_bits_uint {
    ($($prim:ty),*) => {
        $(
            impl<const LEN: usize> TryBits for UInt<$prim, LEN>
            where
                $prim: IsStorageForBits<LEN>,
            {
                type Bits = Self;

                #[inline(always)]
                fn try_from_bits(value: Self::Bits) -> Option<Self> {
                    Some(value)
                }

                #[inline(always)]
                fn into_bits(self) -> Self::Bits {
                    self
                }
            }

            impl<const LEN: usize> Bits for UInt<$prim, LEN>
            where
                $prim: IsStorageForBits<LEN>,
            {
                #[inline(always)]
                fn from_bits(value: Self::Bits) -> Self {
                    value
                }
            }

            impl TryBits for $prim {
                type Bits = Self;

                #[inline(always)]
                fn try_from_bits(value: Self::Bits) -> Option<Self> {
                    Some(value)
                }

                #[inline(always)]
                fn into_bits(self) -> Self::Bits {
                    self
                }
            }

            impl Bits for $prim {
                #[inline(always)]
                fn from_bits(value: Self::Bits) -> Self {
                    value
                }
            }
        )*
    };
}

impl_bits_uint!(u8, u16, u32, u64);

macro_rules! impl_bits_sint {
    ($($prim:ty = $uprim:ty),*) => {
        $(
            impl<const LEN: usize> TryBits for SInt<$prim, LEN>
            where
                $prim: IsStorageForBits<LEN>,
                $uprim: IsStorageForBits<LEN>,
            {
                type Bits = UInt<$uprim, LEN>;

                #[inline(always)]
                fn try_from_bits(value: Self::Bits) -> Option<Self> {
                    Some(Self::new(value.value() as $prim))
                }

                #[inline(always)]
                fn into_bits(self) -> Self::Bits {
                    Self::Bits::new(self.value() as $uprim)
                }
            }

            impl<const LEN: usize> Bits for SInt<$prim, LEN>
            where
                $prim: IsStorageForBits<LEN>,
                $uprim: IsStorageForBits<LEN>,
            {
                #[inline(always)]
                fn from_bits(value: Self::Bits) -> Self {
                    Self::new(value.value() as $prim)
                }
            }

            impl TryBits for $prim {
                type Bits = $uprim;

                #[inline(always)]
                fn try_from_bits(value: Self::Bits) -> Option<Self> {
                    Some(value as Self)
                }

                #[inline(always)]
                fn into_bits(self) -> Self::Bits {
                    self as $uprim
                }
            }

            impl Bits for $prim {
                #[inline(always)]
                fn from_bits(value: Self::Bits) -> Self {
                    value as Self
                }
            }
        )*
    };
}

impl_bits_sint!(i8 = u8, i16 = u16, i32 = u32, i64 = u64);

impl TryBits for bool {
    type Bits = integer::u1;

    #[inline(always)]
    fn try_from_bits(value: Self::Bits) -> Option<Self> {
        Some(value.value() == 1)
    }

    #[inline(always)]
    fn into_bits(self) -> Self::Bits {
        integer::u1::new(self.into())
    }
}

impl Bits for bool {
    #[inline(always)]
    fn from_bits(value: Self::Bits) -> Self {
        value.value() == 1
    }
}
