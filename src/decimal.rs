use std::{fmt::Display, str::FromStr};

use num::{BigUint, Zero};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Default)]
pub struct Balance(pub(crate) BigUint);

impl Display for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fractional = &self.0 % 10000u32;
        let integral = &self.0 / 10000u32;
        write!(f, "{}.{:>04}", integral, fractional)
    }
}

#[derive(Debug)]
pub struct DecimalError;

impl FromStr for Balance {
    type Err = DecimalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.find(|c: char| !matches!(c, '0'..='9' | '.')).is_some() {
            return Err(DecimalError);
        }
        if let Some(dp_loc) = s.find(".") {
            const SCALE: &[u32] = &[1, 10, 100, 1000];
            let integral: BigUint = s[..dp_loc].parse().map_err(|_| DecimalError)?;
            let fractional_part = &s[dp_loc + 1..];
            if fractional_part.find('.').is_some() {
                return Err(DecimalError);
            }
            let fractional_part = if fractional_part.len() > 4 {
                &fractional_part[..4]
            } else {
                fractional_part
            };
            let fractional: BigUint = if fractional_part.is_empty() {
                <_>::zero()
            } else {
                BigUint::from_str(fractional_part).map_err(|_| DecimalError)?
                    * SCALE[4 - fractional_part.len()]
            };
            Ok(Self(integral * 10000u32 + fractional))
        } else {
            s.parse()
                .map_err(|_| DecimalError)
                .map(|dec: BigUint| Self(dec * 10000u32))
        }
    }
}

impl<'de> Deserialize<'de> for Balance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse()
            .map_err(|_| Error::custom("invalid decimal specification"))
    }
}

impl Serialize for Balance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_correctly() {
        assert_eq!(Balance(1u8.into()).to_string(), "0.0001");
        assert_eq!(Balance(0u8.into()).to_string(), "0.0000");
        assert_eq!(Balance(10000u32.into()).to_string(), "1.0000");
        assert_eq!(Balance(100001u32.into()).to_string(), "10.0001");
    }

    #[test]
    fn parse_correctly() {
        assert!(Balance::from_str("not a number").is_err());
        assert!(Balance::from_str("  1.100001.  ").is_err());
        assert_eq!(
            Balance::from_str("  1.100001  ").unwrap().0,
            11000u32.into()
        );
        assert_eq!(Balance::from_str("  1.1  ").unwrap().0, 11000u32.into());
        assert_eq!(Balance::from_str("  1. ").unwrap().0, 10000u32.into());
        assert_eq!(Balance::from_str("  1 ").unwrap().0, 10000u32.into());
        assert_eq!(Balance::from_str("  0 ").unwrap().0, 0u32.into());
        assert_eq!(Balance::from_str("  10 ").unwrap().0, 100000u32.into());
    }
}
