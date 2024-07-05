use base64::Engine;

#[must_use]
pub fn to_base64_padded<T>(data: T) -> String
where
    T: AsRef<[u8]>,
{
    base64::engine::general_purpose::URL_SAFE.encode(data)
}

pub fn from_base64_padded<T>(data: T) -> Result<Vec<u8>, base64::DecodeError>
where
    T: AsRef<[u8]>,
{
    base64::engine::general_purpose::URL_SAFE.decode(data)
}

#[must_use]
pub fn to_base64<T>(data: T) -> String
where
    T: AsRef<[u8]>,
{
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

pub fn from_base64<T>(data: T) -> Result<Vec<u8>, base64::DecodeError>
where
    T: AsRef<[u8]>,
{
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data)
}

pub const BASE36_CHARS: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

pub trait BaseEncoding: Sized {
    fn to_base(self, base: u32) -> String;
    fn convert_from_base(str: &str, base: u32) -> Result<Self, String>;
}

macro_rules! impl_base_encoding {
    ($type:ty) => {
        impl_base_encoding!($type, u32);
    };
    ($type:ty, $base_type:ty) => {
        impl BaseEncoding for $type {
            fn to_base(self, base: $base_type) -> String {
                assert!(
                    base as usize <= BASE36_CHARS.len(),
                    "Invalid base: {}",
                    base
                );

                let base = Self::try_from(base).expect("Should be valid base");

                let mut result = String::new();
                let mut data = self;

                while data > 0 {
                    result.push(
                        BASE36_CHARS[usize::try_from(data % base).expect("Should be valid index")],
                    );
                    data /= base;
                }

                result.chars().rev().collect()
            }

            fn convert_from_base(s: &str, base: $base_type) -> Result<Self, String> {
                Self::from_str_radix(s, base).map_err(|e| e.to_string())
            }
        }
    };
}

impl_base_encoding!(u128);
impl_base_encoding!(u64);
impl_base_encoding!(u32);
impl_base_encoding!(u16);
impl_base_encoding!(u8);

impl_base_encoding!(i128);
impl_base_encoding!(i64);
impl_base_encoding!(i32);
impl_base_encoding!(i16);
impl_base_encoding!(i8);
