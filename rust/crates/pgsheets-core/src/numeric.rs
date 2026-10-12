//! How a value is read as a number, and how a number is laid out as decimal digits.
//!
//! Both directions of this file exist because Rust's own number handling is *not* a drop-in
//! replacement for .NET's, and the difference is observable in cell contents:
//!
//! * `f64::from_str` rejects `"1,000"` and surrounding whitespace, both of which
//!   `double.TryParse` with `NumberStyles.Float | NumberStyles.AllowThousands` accepts.
//! * Rust's `{:.2}` rounds the exact binary value half-to-even; .NET's `Math.Round` on a `decimal`
//!   rounds half-away-from-zero on a *decimal* expansion. `1.005` renders as `1.01` under .NET and
//!   as `1.00` under `{:.2}`, because the nearest `f64` to 1.005 is a hair below it.
//!
//! So a value is decomposed into the 15 significant decimal digits that .NET's double-to-decimal
//! conversion rounds to, and every later step — percent scaling, thousand scaling, rounding,
//! padding, grouping, the `G15` general rendering — works on those digits exactly.

/// A finite value split into decimal digits and an exponent.
///
/// `value = ±(<digits[0]>.<digits[1..]>) × 10^exp`, with no trailing zero digits and at least one
/// digit held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecimalDigits {
    negative: bool,
    digits: Vec<u8>,
    exp: i32,
}

impl DecimalDigits {
    /// Number of significant digits kept. Fifteen is what .NET rounds a `double` to when it converts
    /// one to `decimal`, and matching that width is what keeps the rounding identical.
    const SIGNIFICANT_DIGITS: usize = 15;

    /// Decomposes a finite value; `None` for NaN and the infinities, which have no digit form.
    pub(crate) fn from_f64(value: f64) -> Option<Self> {
        Self::decompose(value, Self::SIGNIFICANT_DIGITS - 1)
    }

    /// Decomposes a value into the *shortest* digits that round-trip, which is what .NET's `"R"`
    /// format writes and therefore what a rewritten formula has to contain.
    ///
    /// Fifteen significant digits is the right width for reading a value, but the wrong width for
    /// writing one back: `1.005` has to survive as `1.005`, and a computed `0.30000000000000004` has
    /// to survive as itself rather than as `0.3`.
    pub(crate) fn from_shortest(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }

        // Rust's own `{:e}` is correctly rounded to the shortest round-tripping digits, which is the
        // same guarantee .NET's "R" makes.
        let text = format!("{value:e}");
        let (mantissa, exponent) = text.split_once('e')?;
        let exp: i32 = exponent.parse().ok()?;

        let mut digits: Vec<u8> = mantissa
            .bytes()
            .filter(u8::is_ascii_digit)
            .map(|byte| byte - b'0')
            .collect();

        while digits.len() > 1 && digits.last() == Some(&0) {
            digits.pop();
        }

        Some(Self {
            negative: value.is_sign_negative(),
            digits,
            exp,
        })
    }

    /// Decomposes a finite value by rendering the digits at a fixed width.
    fn decompose(value: f64, digits_after_point: usize) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }

        // `{:.14e}` is exactly 15 significant digits, correctly rounded, in the form `1.005e0`.
        let text = format!("{:.*e}", digits_after_point, value.abs());
        let (mantissa, exponent) = text.split_once('e')?;
        let exp: i32 = exponent.parse().ok()?;

        let mut digits: Vec<u8> = mantissa
            .bytes()
            .filter(u8::is_ascii_digit)
            .map(|byte| byte - b'0')
            .collect();

        while digits.len() > 1 && digits.last() == Some(&0) {
            digits.pop();
        }

        Some(Self {
            negative: value.is_sign_negative(),
            digits,
            exp,
        })
    }

    /// True when every digit is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.digits.iter().all(|digit| *digit == 0)
    }

    /// Multiplies by 10^`power`, which only moves the exponent: `%` and the trailing-comma scale of a
    /// format are exact, and staying exact is the whole point of this type.
    pub(crate) fn scale_by_power_of_ten(&mut self, power: i32) {
        self.exp += power;
    }

    /// Rounds to `places` digits after the decimal point, half away from zero.
    pub(crate) fn round_to_places(&mut self, places: u32) {
        // The digit at this index is the first one the rounded result drops.
        let keep = self.exp + 1 + places as i32;

        if keep >= self.digits.len() as i32 {
            return;
        }

        if keep <= 0 {
            // Every digit is dropped. At exactly zero the leading digit decides between 0 and one unit
            // of the last place, which is what turns 0.5 under "0" into 1.
            let round_up = keep == 0 && self.digits[0] >= 5;

            self.digits.clear();

            if round_up {
                self.digits.push(1);
                self.exp += 1;
            } else {
                self.digits.push(0);
                self.exp = 0;
            }

            return;
        }

        let round_up = self.digits[keep as usize] >= 5;

        self.digits.truncate(keep as usize);

        if round_up {
            self.round_up();
        }

        while self.digits.len() > 1 && self.digits.last() == Some(&0) {
            self.digits.pop();
        }
    }

    /// Adds one to the least significant digit, carrying as far as it has to.
    fn round_up(&mut self) {
        let mut index = self.digits.len();

        while index > 0 {
            index -= 1;

            if self.digits[index] < 9 {
                self.digits[index] += 1;
                return;
            }

            self.digits[index] = 0;

            if index == 0 {
                // 9.99 rounded to one place is 10.0: the carry grows a digit, and the exponent with it.
                self.digits.insert(0, 1);
                self.exp += 1;
                return;
            }
        }
    }

    /// Lays the digits out with a fixed number of decimals, the way a number-format section asks for
    /// them: at least `min_int` integer digits, exactly `max_frac` decimals before trailing zeros are
    /// dropped down to `min_frac`.
    pub(crate) fn to_fixed(
        &self,
        min_int: usize,
        min_frac: usize,
        max_frac: usize,
        grouping: bool,
    ) -> String {
        let integer_digits = self.exp + 1;
        let mut integer = String::new();

        if integer_digits <= 0 {
            integer.push('0');
        } else {
            let available = (integer_digits as usize).min(self.digits.len());

            for digit in &self.digits[..available] {
                integer.push(digit_char(*digit));
            }

            // 1 × 10^2 is "100": positions past the last held digit are zeros.
            for _ in available..integer_digits as usize {
                integer.push('0');
            }
        }

        while integer.len() < min_int {
            integer.insert(0, '0');
        }

        let mut fraction = String::new();

        for step in 0..max_frac as i32 {
            let position = integer_digits + step;

            if position >= 0 && (position as usize) < self.digits.len() {
                fraction.push(digit_char(self.digits[position as usize]));
            } else {
                fraction.push('0');
            }
        }

        while fraction.len() > min_frac && fraction.ends_with('0') {
            fraction.pop();
        }

        let mut text = String::new();

        if grouping {
            text.push_str(&group_digits(&integer));
        } else {
            text.push_str(&integer);
        }

        if !fraction.is_empty() {
            text.push('.');
            text.push_str(&fraction);
        }

        text
    }

    /// The general rendering, .NET's `G15`: fifteen significant digits, scientific notation once the
    /// exponent falls outside the range a plain decimal can carry.
    pub(crate) fn to_general(&self) -> String {
        let mut text = String::new();

        if self.negative {
            text.push('-');
        }

        if self.is_zero() {
            text.push('0');
            return text;
        }

        // .NET switches to "E" notation below 10^-5 and from 10^15 up.
        if self.exp < -4 || self.exp >= Self::SIGNIFICANT_DIGITS as i32 {
            text.push(digit_char(self.digits[0]));

            if self.digits.len() > 1 {
                text.push('.');
                text.push_str(&digit_string(&self.digits[1..]));
            }

            text.push('E');
            text.push(if self.exp < 0 { '-' } else { '+' });
            text.push_str(&format!("{:02}", self.exp.abs()));
            return text;
        }

        let integer_digits = self.exp + 1;

        if integer_digits <= 0 {
            text.push_str("0.");

            for _ in 0..-integer_digits {
                text.push('0');
            }

            text.push_str(&digit_string(&self.digits));
        } else if integer_digits as usize >= self.digits.len() {
            text.push_str(&digit_string(&self.digits));

            for _ in self.digits.len()..integer_digits as usize {
                text.push('0');
            }
        } else {
            let split = integer_digits as usize;
            text.push_str(&digit_string(&self.digits[..split]));
            text.push('.');
            text.push_str(&digit_string(&self.digits[split..]));
        }

        text
    }
}

/// Reads a number the way `double.TryParse` does with `Float | AllowThousands`: surrounding
/// whitespace is ignored and group separators are decoration rather than digits.
pub(crate) fn parse_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains(',') {
        let cleaned: String = trimmed.chars().filter(|ch| *ch != ',').collect();
        return cleaned.parse::<f64>().ok();
    }

    trimmed.parse::<f64>().ok()
}

/// The general rendering of a number, which is what an unformatted cell shows.
pub(crate) fn general_number(value: f64) -> String {
    match DecimalDigits::from_f64(value) {
        Some(digits) => digits.to_general(),
        None if value.is_nan() => "NaN".to_string(),
        None if value.is_sign_negative() => "-Infinity".to_string(),
        None => "Infinity".to_string(),
    }
}

/// A number written so that reading it back gives exactly the same value.
///
/// This is what `.NET`'s `"R"` format produces, and it is what a rewritten formula has to contain: a
/// formula that is copied, or whose rows are renumbered, is re-rendered from its parse tree, and the
/// numbers in it have to survive the trip unchanged. Rust's own `{}` is not a substitute — it never
/// uses an exponent, so `1e30` would come back as thirty-one digits.
pub fn write_round_trip(value: f64) -> String {
    match DecimalDigits::from_shortest(value) {
        Some(digits) => digits.to_general(),
        None if value.is_nan() => "NaN".to_string(),
        None if value.is_sign_negative() => "-Infinity".to_string(),
        None => "Infinity".to_string(),
    }
}

fn digit_char(digit: u8) -> char {
    (b'0' + digit) as char
}

fn digit_string(digits: &[u8]) -> String {
    digits.iter().map(|digit| digit_char(*digit)).collect()
}

/// Inserts group separators every three digits, counting from the right.
fn group_digits(digits: &str) -> String {
    if digits.len() <= 3 {
        return digits.to_string();
    }

    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    let leading = digits.len() % 3;

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (index - leading).is_multiple_of(3) {
            grouped.push(',');
        }

        grouped.push(digit);
    }

    grouped
}
