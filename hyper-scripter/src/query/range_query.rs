use crate::error::{
    DisplayError, DisplayResult, Error, FormatCode::RangeQuery as RangeQueryCode, Result,
};
use crate::impl_ser_by_to_string;
use std::num::NonZeroU64;
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct RangeQuery {
    min: NonZeroU64,
    max: Option<NonZeroU64>,
}

const SEP: &str = "..";

fn parse_int(s: &str) -> Result<NonZeroU64> {
    let num: NonZeroU64 = s.parse().map_err(|e| {
        Error::Format(RangeQueryCode, s.to_owned()).context(format!("解析整數錯誤 {}", e))
    })?;
    Ok(num)
}

impl RangeQuery {
    pub fn get_max(&self) -> Option<NonZeroU64> {
        self.max
    }
    pub fn get_min(&self) -> NonZeroU64 {
        self.min
    }
}

impl FromStr for RangeQuery {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        if let Some((first, second)) = s.split_once(SEP) {
            if first.is_empty() && second.is_empty() {
                return Err(Error::Format(RangeQueryCode, s.to_owned())
                    .context("不可前後皆為空")
                    .into());
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
                    return Err(Error::Format(RangeQueryCode, s.to_owned())
                        .context("max 不可小於等於 min")
                        .into());
                }
                Some(max)
            };
            Ok(RangeQuery { min, max })
        } else {
            let num = parse_int(s)?;
            Ok(RangeQuery {
                min: num,
                max: Some(NonZeroU64::new(num.get() + 1).unwrap()),
            })
        }
    }
}

impl std::fmt::Display for RangeQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let RangeQuery { min, max } = *self;
        if Some(min) == max {
            write!(f, "{}", min)?;
        } else {
            write!(f, "{}{}", min, SEP)?;
            if let Some(max) = max {
                write!(f, "{}", max)?;
            }
        }
        Ok(())
    }
}

impl_ser_by_to_string!(RangeQuery);
