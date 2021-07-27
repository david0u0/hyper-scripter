use crate::error::{Error, FormatCode::RangeQuery as RangeQueryCode, Result};
use serde::Serialize;
use std::num::NonZeroU64;
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub enum RangeQuery {
    Range {
        min: NonZeroU64,
        max: Option<NonZeroU64>,
    },
    Single(NonZeroU64),
}

const SEP: &str = "..";

fn parse_int(s: &str) -> Result<NonZeroU64> {
    let num: NonZeroU64 = s.parse().map_err(|e| {
        Error::Format(RangeQueryCode, s.to_owned()).context(format!("解析整數錯誤 {}", e))
    })?;
    Ok(num)
}

impl FromStr for RangeQuery {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        if let Some((first, second)) = s.split_once(SEP) {
            if first.is_empty() && second.is_empty() {
                return Err(Error::Format(RangeQueryCode, s.to_owned()).context("不可前後皆為空"));
            }
            let min = if first.is_empty() {
                NonZeroU64::new(1).unwrap()
            } else {
                parse_int(first)?
            };
            let max = if second.is_empty() {
                None
            } else {
                let max = parse_int(second)?;
                if max <= min {
                    return Err(
                        Error::Format(RangeQueryCode, s.to_owned()).context("max 不可小於等於 min")
                    );
                }
                Some(max)
            };
            Ok(RangeQuery::Range { min, max })
        } else {
            let num = parse_int(s)?;
            Ok(RangeQuery::Single(num))
        }
    }
}

impl std::fmt::Display for RangeQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RangeQuery::Single(n) => write!(f, "{}", n),
            RangeQuery::Range { min, max } => {
                write!(f, "{}..", min)?;
                if let Some(max) = max {
                    write!(f, "{}", max)?;
                }
                Ok(())
            }
        }
    }
}

impl Serialize for RangeQuery {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}
