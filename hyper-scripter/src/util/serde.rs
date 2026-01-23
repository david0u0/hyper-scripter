macro_rules! impl_ser_by_to_string {
    ($target:ty) => {
        impl serde::Serialize for $target {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
    };
}
macro_rules! impl_ser_and_display_by_as_ref {
    ($target:ty) => {
        impl std::fmt::Display for $target {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_ref())
            }
        }
        impl serde::Serialize for $target {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_ref())
            }
        }
    };
}

macro_rules! impl_de_by_from_str {
    ($target:ty) => {
        impl<'de> serde::Deserialize<'de> for $target {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: &str = serde::Deserialize::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

pub(crate) use impl_de_by_from_str;
pub(crate) use impl_ser_and_display_by_as_ref;
pub(crate) use impl_ser_by_to_string;
