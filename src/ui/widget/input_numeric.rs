use {
    crate::utils::numeric::{
        Abs,
        Bounded,
        BoundedFrom,
    },
    super::{
        InputField,
        InputFieldDisplay,
        InputFieldMemory,
        InputFieldProxy,
    },
    std::{
        cmp::{
            min,
            max
        },
        hash::Hash,
        fmt::{
            Display,
            LowerExp,
        },
        ops::RangeInclusive,
        rc::Rc,
        str::{
            self,
            FromStr,
        },
    },
    eframe::egui::*,
};

/*
 * TODO:
 * ~ Cache button padding
 * ~ Min. precision applied numbers w/o decimal
 */

pub enum NumericDefault<N> {
    Number(N),
    String(String),
    RcString(Rc<String>),
    None,
}

pub trait Numeric:
    Abs + Bounded + BoundedFrom + Copy + Display + FromStr + Into<f64> + LowerExp + PartialOrd + ToString
{}

pub struct NumericInputField<'a, N> {
    clamp_range: RangeInclusive<N>,
    default: NumericDefault<N>,
    id: Id,
    id_salt: Option<Id>,
    max_precision: usize,
    min_precision: usize,
    prefix: &'a str,
    proxy: InputFieldProxy<'a, N>,
    scientific: bool,
    signed: bool,
    suffix: &'a str,
}

impl<N> Numeric for N where N:
    Abs + Bounded + BoundedFrom + Copy + Display + FromStr + Into<f64> + LowerExp + PartialOrd + ToString
{}

impl<'a, N> NumericInputField<'a, N>
    where N: Numeric
{
    pub fn new(id: Id, target: &'a mut N) -> Self {
        Self {
            clamp_range: N::bounds(),
            default: NumericDefault::None,
            id: id,
            id_salt: None,
            max_precision: 10,
            min_precision: 0,
            prefix: "",
            proxy: Box::new(move |value_opt| {
                if let Some(value) = value_opt {
                    *target = value;
                }

                *target
            }),
            scientific: false,
            signed: false,
            suffix: "",
        }
    }

    /* Builder methods
     */

    pub fn clamp_range(self, range: RangeInclusive<N>) -> Self {
        Self {
            clamp_range: range,
            ..self
        }
    }

    pub fn default(self, default: Option<NumericDefault<N>>) -> Self {
        Self {
            default: default.unwrap_or(NumericDefault::None),
            ..self
        }
    }

    pub fn max_precision(self, precision: usize) -> Self {
        Self {
            max_precision: max(precision, self.min_precision),
            ..self
        }
    }

    pub fn min_precision(self, precision: usize) -> Self {
        Self {
            min_precision: min(precision, self.max_precision),
            ..self
        }
    }

    pub fn prefix(self, prefix: &'a str) -> Self {
        Self {
            prefix: prefix,
            ..self
        }
    }

    pub fn salt(self, salt: impl Hash) -> Self {
        Self {
            id_salt: Some(Id::new(salt)),
            ..self
        }
    }

    pub fn scientific(self, scientific: bool) -> Self {
        Self {
            min_precision: 0,
            max_precision: 0,
            scientific: scientific,
            ..self
        }
    }

    pub fn signed(self, signed: bool) -> Self {
        Self {
            signed: signed,
            ..self
        }
    }

    pub fn suffix(self, suffix: &'a str) -> Self {
        Self {
            suffix: suffix,
            ..self
        }
    }

    /* Impl. methods
     */

    fn use_scientific(&self, number: N) -> bool {
        let n = number.into().abs();

        self.scientific || (n != 0.0 && n < 10_f64.powf(-(self.max_precision as f64)))
                        || (            n > 10_f64.powf(  self.max_precision as f64))
    }
}

impl<'a, N> InputField<'a> for NumericInputField<'a, N>
    where N: Numeric,
{
    type Input = N;

    fn display(&self, mut number: N) -> String {
        let value_text = match
            (self.use_scientific(number),
             self.signed)
        {
            (true, true) => format!("{number:-e}"),
            (true, false) => format!("{number:e}"),
            (false, true) => format!("{number:-}"),
            _ => as_num_string(number, self.min_precision, self.max_precision)
        };

        format!("{}{}{}", self.prefix, value_text, self.suffix)
    }

    fn normalize(&self, mut number: N) -> N {
        clamp(number, &self.clamp_range)
    }

    fn parse(&self, value_text: &String) -> Option<N> {
        N::bounded_from(value_text).map(|number| self.normalize(number)).ok()
    }

    fn proxy(&mut self) -> &mut InputFieldProxy<'a, Self::Input> {
        &mut self.proxy
    }
}

impl<'a, N> InputFieldDisplay<'a, N> for NumericInputField<'a, N>
    where N: Numeric,
{}

impl<'a, N> InputFieldMemory<'a, N> for NumericInputField<'a, N>
    where N: Numeric,
{
    fn default(&self) -> Option<N> {
        match &self.default {
            NumericDefault::Number(n) => Some(*n),
            NumericDefault::String(s) => self.parse(s),
            NumericDefault::RcString(s) => self.parse(s.as_ref()),
            NumericDefault::None => None,
        }
    }

    fn memory_id(&self) -> Id {
        self.id
    }

    fn widget_id(&self) -> Id {
        if let Some(salt) = self.id_salt {
            self.id.with(salt)
        } else {
            self.id
        }
    }
}

fn as_num_string<N>(number: N, min_precision: usize, max_precision: usize) -> String
    where N: Copy + ToString,
{
    let mut output = number.to_string();

    if let Some(decimal) = output.find('.') {
        if min_precision > 0 {
            output += &"0".repeat(
                min_precision
                    .checked_sub(output.len() - (decimal + 1))
                    .unwrap_or(0)
            );
        }

        if max_precision > 0 {
            round_num_string(&mut output, max_precision);
        }
    }

    output
}

fn clamp<N>(number: N, range: &RangeInclusive<N>) -> N
    where N: Copy + PartialOrd,
{
    if number.le(range.start()) {
        *range.start()
    } else if number.ge(range.end()) {
        *range.end()
    } else {
        number
    }
}

fn round_num_string(string: &mut String, precision: usize) {
    if let Some(decimal) = string.find('.') {
        let roll_index = (decimal + 1) + precision;

        if roll_index < string.len() {
            unsafe {
                let mut bytes = string.as_bytes_mut(); 

                if '5' as u8 <= bytes[roll_index] {
                    *string = round_num_string_impl(&mut bytes, decimal, roll_index);
                } else {
                    *string = String::from(&string[0..roll_index]);
                }
            }
        }
    }
}

fn round_num_string_impl(bytes: &mut [u8], decimal: usize, roll_index: usize) -> String {
    let bytes_as_str;

    for byte in bytes.iter_mut().take(roll_index).rev() {
        if *byte == '9' as u8 {
            *byte = '0' as u8;
        } else if '0' as u8 <= *byte {
            *byte += 1;

            break;
        }
    }

    bytes_as_str = str::from_utf8(&bytes[0..roll_index])
        .unwrap_or("<ROUND FAILURE>")
        .trim_end_matches('.');

    if decimal > 1 {
        if let Some('0') = bytes_as_str.chars().nth(0) {
            String::from("1") + &bytes_as_str
        } else {
            String::from(bytes_as_str)
        }
    } else {
        String::from(bytes_as_str)
    }
}
