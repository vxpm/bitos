use bitut::BitUtils;
use num_traits::PrimInt;
use seq_macro::seq;

#[cfg(feature = "zerocopy")]
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

mod sealed {
    pub type U8 = u8;
    pub type U16 = u16;
    pub type U32 = u32;
    pub type U64 = u64;

    pub type I8 = i8;
    pub type I16 = i16;
    pub type I32 = i32;
    pub type I64 = i64;

    // #[diagnostic::on_unimplemented(
    //     message = "`{Self}` is not a supported integer primitive",
    //     note = "supported integer primitives are u8, u16, u32 and u64"
    // )]
    // pub trait Primitive {}
    // impl Primitive for u8 {}
    // impl Primitive for u16 {}
    // impl Primitive for u32 {}
    // impl Primitive for u64 {}
    //
    // #[diagnostic::on_unimplemented(
    //     message = "`{Self}` is not a supported integer type",
    //     note = "supported integer types are primitives (u8, u16, u32 and u64) and any uN in `bitos::integer`"
    // )]
    // pub trait IntegerMarker {}
    // impl<T> IntegerMarker for T where T: Primitive {}
    // impl<T, const LEN: usize> IntegerMarker for super::UInt<T, LEN> where
    //     T: super::Integer + super::IsStorageForBits<LEN>
    // {
    // }
}

/// Trait for unsigned integer types.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a supported unsigned integer type",
    note = "supported unsigned integer types are primitives (u8, u16, u32 and u64) and any uN in `bitos::integer`"
)]
pub trait UnsignedInt: Copy + TryFrom<u64> + Into<u64> {
    /// The bit width of this integer type.
    const BITS: usize;

    /// Creates a new value of this integer type from an [`u64`]. This is a lossy operation: the
    /// u64 value will be masked to fit within this integer type.
    fn new(value: u64) -> Self;

    /// Returns the value of this integer type represented by an [`u64`].
    #[inline(always)]
    fn value(self) -> u64 {
        self.into()
    }
}

impl UnsignedInt for u8 {
    const BITS: usize = 8;

    #[inline(always)]
    fn new(value: u64) -> Self {
        value as Self
    }
}

impl UnsignedInt for u16 {
    const BITS: usize = 16;

    #[inline(always)]
    fn new(value: u64) -> Self {
        value as Self
    }
}

impl UnsignedInt for u32 {
    const BITS: usize = 32;

    #[inline(always)]
    fn new(value: u64) -> Self {
        value as Self
    }
}

impl UnsignedInt for u64 {
    const BITS: usize = 64;

    #[inline(always)]
    fn new(value: u64) -> Self {
        value
    }
}

#[inline(always)]
const fn unsigned_mask(bits: usize) -> u64 {
    (1 << bits) - 1
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not the correct storage for UInts of bit width {LEN}",
    note = "consider using type aliases defined in `bitos::uint`"
)]
pub trait IsStorageForBits<const LEN: usize> {}

/// An unsigned integer with `LEN` bits.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(
    feature = "zerocopy",
    derive(KnownLayout, Immutable, IntoBytes, FromBytes)
)]
#[repr(transparent)]
pub struct UInt<T, const LEN: usize>(T);

impl<T: std::fmt::Debug, const LEN: usize> std::fmt::Debug for UInt<T, LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: std::fmt::UpperHex, const LEN: usize> std::fmt::UpperHex for UInt<T, LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T, const LEN: usize> UInt<T, LEN>
where
    T: UnsignedInt + PrimInt + IsStorageForBits<LEN>,
{
    #[inline(always)]
    pub fn new(value: T) -> Self {
        Self(value & T::new(const { unsigned_mask(LEN) }))
    }

    #[inline(always)]
    pub fn value(self) -> T {
        let value = self.0;

        unsafe { std::hint::assert_unchecked(value <= T::new(const { unsigned_mask(LEN) })) };
        value
    }
}

pub struct ValueDoesNotFitErr;

impl<T, const LEN: usize> TryFrom<u64> for UInt<T, LEN>
where
    T: UnsignedInt + PrimInt + IsStorageForBits<LEN>,
{
    type Error = ValueDoesNotFitErr;

    #[inline(always)]
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        (value <= const { unsigned_mask(LEN) })
            .then(|| Self::new(T::new(value)))
            .ok_or(ValueDoesNotFitErr)
    }
}

impl<T, const LEN: usize> From<UInt<T, LEN>> for u64
where
    T: UnsignedInt + PrimInt + IsStorageForBits<LEN>,
{
    #[inline(always)]
    fn from(value: UInt<T, LEN>) -> u64 {
        value.value().into()
    }
}

impl<T, const LEN: usize> UnsignedInt for UInt<T, LEN>
where
    T: UnsignedInt + PrimInt + IsStorageForBits<LEN>,
{
    const BITS: usize = LEN;

    #[inline(always)]
    fn new(value: u64) -> Self {
        Self::new(T::new(value))
    }
}

impl<T, const LEN: usize> BitUtils for UInt<T, LEN>
where
    T: UnsignedInt + PrimInt + BitUtils + IsStorageForBits<LEN>,
{
    #[inline(always)]
    fn bit(self, index: u8) -> bool {
        self.0.bit(index)
    }

    #[inline(always)]
    fn try_bit(self, index: u8) -> Option<bool> {
        self.0.try_bit(index)
    }

    #[inline(always)]
    fn with_bit(self, index: u8, value: bool) -> Self {
        Self::new(self.0.with_bit(index, value))
    }

    #[inline(always)]
    fn try_with_bit(self, index: u8, value: bool) -> Option<Self> {
        self.0.try_with_bit(index, value).map(Self::new)
    }

    #[inline(always)]
    fn bits(self, start: u8, end: u8) -> Self {
        Self::new(self.0.bits(start, end))
    }

    #[inline(always)]
    fn try_bits(self, start: u8, end: u8) -> Option<Self> {
        self.0.try_bits(start, end).map(Self::new)
    }

    #[inline(always)]
    fn with_bits(self, start: u8, end: u8, value: Self) -> Self {
        Self::new(self.0.with_bits(start, end, value.value()))
    }

    #[inline(always)]
    fn try_with_bits(self, start: u8, end: u8, value: Self) -> Option<Self> {
        self.0
            .try_with_bits(start, end, value.value())
            .map(Self::new)
    }
}

seq!(N in 1..8 {
    #(
        #[allow(non_camel_case_types)]
        pub type u~N = UInt<u8, N>;

        impl IsStorageForBits<N> for u8 {}
    )*
});

#[allow(non_camel_case_types)]
pub type u8 = sealed::U8;

seq!(N in 9..16 {
    #(
        #[allow(non_camel_case_types)]
        pub type u~N = UInt<u16, N>;

        impl IsStorageForBits<N> for u16 {}
    )*
});

#[allow(non_camel_case_types)]
pub type u16 = sealed::U16;

seq!(N in 17..32 {
    #(
        #[allow(non_camel_case_types)]
        pub type u~N = UInt<u32, N>;

        impl IsStorageForBits<N> for u32 {}
    )*
});

#[allow(non_camel_case_types)]
pub type u32 = sealed::U32;

seq!(N in 33..64 {
    #(
        #[allow(non_camel_case_types)]
        pub type u~N = UInt<u64, N>;

        impl IsStorageForBits<N> for u64 {}
    )*
});

#[allow(non_camel_case_types)]
pub type u64 = sealed::U64;

/// Trait for signed integer types.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a supported signed integer type",
    note = "supported signed integer types are primitives (i8, i16, i32 and i64) and any iN in `bitos::integer`"
)]
pub trait SignedInt: Copy + TryFrom<i64> + Into<i64> {
    /// The bit width of this integer type.
    const BITS: usize;

    /// Creates a new value of this integer type from an [`i64`]. This is a lossy operation: the
    /// u64 value will be masked to fit within this integer type.
    fn new(value: i64) -> Self;

    /// Returns the value of this integer type represented by an [`i64`].
    #[inline(always)]
    fn value(self) -> i64 {
        self.into()
    }
}

impl SignedInt for i8 {
    const BITS: usize = 8;

    #[inline(always)]
    fn new(value: i64) -> Self {
        value as Self
    }
}

impl SignedInt for i16 {
    const BITS: usize = 16;

    #[inline(always)]
    fn new(value: i64) -> Self {
        value as Self
    }
}

impl SignedInt for i32 {
    const BITS: usize = 32;

    #[inline(always)]
    fn new(value: i64) -> Self {
        value as Self
    }
}

impl SignedInt for i64 {
    const BITS: usize = 64;

    #[inline(always)]
    fn new(value: i64) -> Self {
        value
    }
}

#[inline(always)]
const fn signed_mask(bits: usize) -> i64 {
    unsigned_mask(bits) as i64
}

/// A signed integer with `LEN` bits.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(
    feature = "zerocopy",
    derive(KnownLayout, Immutable, IntoBytes, FromBytes)
)]
#[repr(transparent)]
pub struct SInt<T, const LEN: usize>(T);

impl<T: std::fmt::Debug, const LEN: usize> std::fmt::Debug for SInt<T, LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: std::fmt::UpperHex, const LEN: usize> std::fmt::UpperHex for SInt<T, LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T, const LEN: usize> SInt<T, LEN>
where
    T: SignedInt + PrimInt + IsStorageForBits<LEN>,
{
    #[inline(always)]
    pub fn new(value: T) -> Self {
        let masked = value & T::new(const { signed_mask(LEN) });

        let rem = T::BITS - LEN;
        let sign_extended = (masked << rem) >> rem;

        Self(sign_extended)
    }

    #[inline(always)]
    pub fn value(self) -> T {
        let value = self.0;

        let max = const { signed_mask(LEN - 1) };
        let min = const { !signed_mask(LEN - 1) };

        unsafe { std::hint::assert_unchecked(value <= T::new(max)) };
        unsafe { std::hint::assert_unchecked(value >= T::new(min)) };
        value
    }
}

impl<T, const LEN: usize> BitUtils for SInt<T, LEN>
where
    T: SignedInt + PrimInt + BitUtils + IsStorageForBits<LEN>,
{
    #[inline(always)]
    fn bit(self, index: u8) -> bool {
        self.0.bit(index)
    }

    #[inline(always)]
    fn try_bit(self, index: u8) -> Option<bool> {
        self.0.try_bit(index)
    }

    #[inline(always)]
    fn with_bit(self, index: u8, value: bool) -> Self {
        Self::new(self.0.with_bit(index, value))
    }

    #[inline(always)]
    fn try_with_bit(self, index: u8, value: bool) -> Option<Self> {
        self.0.try_with_bit(index, value).map(Self::new)
    }

    #[inline(always)]
    fn bits(self, start: u8, end: u8) -> Self {
        Self::new(self.0.bits(start, end))
    }

    #[inline(always)]
    fn try_bits(self, start: u8, end: u8) -> Option<Self> {
        self.0.try_bits(start, end).map(Self::new)
    }

    #[inline(always)]
    fn with_bits(self, start: u8, end: u8, value: Self) -> Self {
        Self::new(self.0.with_bits(start, end, value.value()))
    }

    #[inline(always)]
    fn try_with_bits(self, start: u8, end: u8, value: Self) -> Option<Self> {
        self.0
            .try_with_bits(start, end, value.value())
            .map(Self::new)
    }
}

seq!(N in 1..8 {
    #(
        #[allow(non_camel_case_types)]
        pub type i~N = SInt<i8, N>;

        impl IsStorageForBits<N> for i8 {}
    )*
});

#[allow(non_camel_case_types)]
pub type i8 = sealed::I8;

seq!(N in 9..16 {
    #(
        #[allow(non_camel_case_types)]
        pub type i~N = SInt<i16, N>;

        impl IsStorageForBits<N> for i16 {}
    )*
});

#[allow(non_camel_case_types)]
pub type i16 = sealed::I16;

seq!(N in 17..32 {
    #(
        #[allow(non_camel_case_types)]
        pub type i~N = SInt<i32, N>;

        impl IsStorageForBits<N> for i32 {}
    )*
});

#[allow(non_camel_case_types)]
pub type i32 = sealed::I32;

seq!(N in 33..64 {
    #(
        #[allow(non_camel_case_types)]
        pub type i~N = SInt<i64, N>;

        impl IsStorageForBits<N> for i64 {}
    )*
});

#[allow(non_camel_case_types)]
pub type i64 = sealed::I64;
