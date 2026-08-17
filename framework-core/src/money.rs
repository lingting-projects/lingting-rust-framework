use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specta::{Type, Types, datatype::DataType};
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// 以分为最小单位的精确金额。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Self = Self(0);

    pub const fn from_cents(cents: i64) -> Self {
        Self(cents)
    }
    pub const fn cents(self) -> i64 {
        self.0
    }
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    pub fn from_yuan_str(value: &str) -> Result<Self, MoneyParseError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(MoneyParseError);
        }
        let negative = value.starts_with('-');
        let value = value.strip_prefix(['-', '+']).unwrap_or(value);
        let (yuan, fraction) = value.split_once('.').unwrap_or((value, ""));
        if yuan.is_empty()
            || !yuan.bytes().all(|byte| byte.is_ascii_digit())
            || fraction.len() > 2
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(MoneyParseError);
        }
        let yuan = yuan.parse::<i64>().map_err(|_| MoneyParseError)?;
        let fraction = match fraction.len() {
            0 => 0,
            1 => (fraction.as_bytes()[0] - b'0') as i64 * 10,
            _ => fraction.parse().map_err(|_| MoneyParseError)?,
        };
        let cents = yuan
            .checked_mul(100)
            .and_then(|value| value.checked_add(fraction))
            .ok_or(MoneyParseError)?;
        Ok(Self(if negative { -cents } else { cents }))
    }
}

impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        Self::from_yuan_str(&string).map_err(serde::de::Error::custom)
    }
}

impl Type for Money {
    fn definition(types: &mut Types) -> DataType {
        String::definition(types)
    }
}

impl Display for Money {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let cents = self.0.unsigned_abs();
        if self.0 < 0 {
            formatter.write_str("-")?;
        }
        write!(formatter, "{}.{:02}", cents / 100, cents % 100)
    }
}

impl Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
impl Neg for Money {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MoneyParseError;
impl Display for MoneyParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("金额格式无效")
    }
}
impl std::error::Error for MoneyParseError {}
