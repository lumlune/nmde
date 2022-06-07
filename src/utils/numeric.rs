use {
    std::{
        num::{
            IntErrorKind,
            ParseIntError,
            ParseFloatError,
        },
        ops::RangeInclusive,
        str::FromStr,
    }
};

pub trait Abs {
    fn abs(self) -> Self;
}

pub trait Bounded: Sized {
    fn bounds() -> RangeInclusive<Self> {
        Self::min_bound()..=Self::max_bound()
    }

    fn min_bound() -> Self;
    fn max_bound() -> Self;
}

pub trait BoundedFrom: FromStr {
    fn bounded_from(string: &String) -> Result<Self, <Self as FromStr>::Err>;
}

/// Temporary (?) protection from panic on signed MIN abs attempt
impl Abs for u8  { fn abs(self) -> Self { self } }
impl Abs for i8  { fn abs(self) -> Self { i8::abs(self.max(Self::MIN + 1)) } }
impl Abs for u16 { fn abs(self) -> Self { self } }
impl Abs for i16 { fn abs(self) -> Self { i16::abs(self.max(Self::MIN + 1)) } }
impl Abs for u32 { fn abs(self) -> Self { self } }
impl Abs for i32 { fn abs(self) -> Self { i32::abs(self.max(Self::MIN + 1)) } }
impl Abs for f32 { fn abs(self) -> Self { f32::abs(self) } }
impl Abs for f64 { fn abs(self) -> Self { f64::abs(self) } }

impl Bounded for u8  { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }
impl Bounded for i8  { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }
impl Bounded for u16 { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }
impl Bounded for i16 { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }
impl Bounded for u32 { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }
impl Bounded for i32 { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }
impl Bounded for f32 { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }
impl Bounded for f64 { fn min_bound() -> Self { Self::MIN } fn max_bound() -> Self { Self::MAX } }

impl BoundedFrom for i8  { fn bounded_from(string: &String) -> Result<Self, ParseIntError> { bounded_int_from(string) } }
impl BoundedFrom for u8  { fn bounded_from(string: &String) -> Result<Self, ParseIntError> { bounded_int_from(string) } }
impl BoundedFrom for i16 { fn bounded_from(string: &String) -> Result<Self, ParseIntError> { bounded_int_from(string) } }
impl BoundedFrom for u16 { fn bounded_from(string: &String) -> Result<Self, ParseIntError> { bounded_int_from(string) } }
impl BoundedFrom for i32 { fn bounded_from(string: &String) -> Result<Self, ParseIntError> { bounded_int_from(string) } }
impl BoundedFrom for u32 { fn bounded_from(string: &String) -> Result<Self, ParseIntError> { bounded_int_from(string) } }
impl BoundedFrom for f32 { fn bounded_from(string: &String) -> Result<Self, ParseFloatError> { string.parse() } }
impl BoundedFrom for f64 { fn bounded_from(string: &String) -> Result<Self, ParseFloatError> { string.parse() } }

fn bounded_int_from<N>(string: &String) -> Result<N, ParseIntError>
    where N: Bounded + FromStr<Err = ParseIntError>,
{
    use IntErrorKind::*;

    string.parse::<N>().map_or_else(|error| {
        match error.kind() {
            PosOverflow => Ok(N::max_bound()),
            NegOverflow => Ok(N::min_bound()),
            _           => Err(error)
        }
    }, |value| {
        Ok(value)
    })
}
