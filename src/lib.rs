use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::{self, Display};
use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};
use std::io::BufRead;
use std::str::FromStr;
use std::string::ToString;

type Result<T> = std::result::Result<T, BencodeError>;

#[derive(Debug)]
pub enum BencodeError {
    Error(String),
    Io(std::io::Error),
    Eof(),
    Parse(std::num::ParseIntError),
}

impl Display for BencodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BencodeError::Error(s) => write!(f, "Bencode Error: {} ", s),
            BencodeError::Io(e) => write!(f, "Bencode Io: {}", e),
            BencodeError::Parse(e) => write!(f, "Bencode Parse: {}", e),
            BencodeError::Eof() => write!(f, "Bencode Eof"),
        }
    }
}

impl From<std::io::Error> for BencodeError {
    fn from(err: std::io::Error) -> BencodeError {
        BencodeError::Io(err)
    }
}

impl From<std::num::ParseIntError> for BencodeError {
    fn from(err: std::num::ParseIntError) -> BencodeError {
        BencodeError::Parse(err)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Map(HashMap<Value, Value>),
    List(Vec<Value>),
    Str(String),
    Int(i32),
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Map(map) => {
                let mut seed = 1;
                for elem in map.iter() {
                    let mut hasher = DefaultHasher::new();
                    elem.hash(&mut hasher);
                    seed = hasher.finish().wrapping_add(seed);
                }
                seed.to_be_bytes().hash(state);
            }
            Value::List(vec) => {
                vec.hash(state);
            }
            Value::Str(s) => {
                s.hash(state);
            }
            Value::Int(i) => {
                i.hash(state);
            }
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

impl From<HashMap<Value, Value>> for Value {
    fn from(m: HashMap<Value, Value>) -> Self {
        Value::Map(m)
    }
}

impl From<HashMap<&str, &str>> for Value {
    fn from(map: HashMap<&str, &str>) -> Self {
        let mut m = HashMap::new();
        for (k, v) in map {
            m.insert(Value::Str(k.to_string()), Value::Str(v.to_string()));
        }
        Value::Map(m)
    }
}

impl TryInto<HashMap<String, String>> for Value {
    type Error = BencodeError;

    fn try_into(self) -> std::result::Result<HashMap<String, String>, Self::Error> {
        match self {
            Value::Map(hm) => {
                let mut map = HashMap::<String, String>::new();
                for key in hm.keys() {
                    // safe to unwrap here
                    map.insert(format!("{}", &key), format!("{}", &hm.get(key).unwrap()));
                }
                Ok(map)
            }
            _ => Err(BencodeError::Error("Expected HashMap Value".into())),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Map(map) => {
                let mut result = String::from("{");
                for (key, val) in map.iter() {
                    result.push_str(&format!("{} {} ", &key, &val));
                }
                let mut result = result.trim_end().to_string();
                result.push('}');
                write!(f, "{}", result)
            }
            Value::List(v) => {
                let mut result = String::from("[");
                for item in v {
                    result.push_str(&item.to_string());
                    result.push_str(", ");
                }
                let mut result = result.trim_end_matches([',', ' ']).to_string();
                result.push(']');
                write!(f, "{}", result)
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::Int(i) => write!(f, "{}", i),
        }
    }
}

impl Value {
    pub fn to_bencode(&self) -> String {
        match self {
            Value::Map(hm) => {
                let mut result = String::from("d");
                for (key, val) in hm.iter() {
                    result.push_str(&format!("{}{}", key.to_bencode(), val.to_bencode()));
                }
                result.push('e');
                result
            }
            Value::List(v) => {
                let mut result = String::from("l");
                for item in v {
                    result.push_str(&item.to_bencode());
                }
                result.push('e');
                result
            }
            Value::Str(s) => format!("{}:{}", s.len(), s),
            Value::Int(i) => format!("i{}e", i),
        }
    }
}

pub fn parse_bencode(reader: &mut dyn BufRead) -> Result<Option<Value>> {
    let mut buf = vec![0; 1];
    // buf.resize(1, 0);
    match reader.read_exact(&mut buf[0..1]) {
        Ok(()) => match buf[0] {
            b'i' => {
                let cnt = reader.read_until(b'e', &mut buf)?;
                let n = i32::from_str(&String::from_utf8_lossy(&buf[1..cnt]))?;
                Ok(Some(Value::Int(n)))
            }
            b'd' => {
                let mut map = HashMap::new();
                loop {
                    match parse_bencode(reader)? {
                        None => return Ok(Some(Value::Map(map))),
                        Some(key) => match parse_bencode(reader)? {
                            Some(val) => {
                                map.insert(key, val);
                            }
                            None => {
                                return Err(BencodeError::Error(
                                    "Map is missing value for key".to_string(),
                                ))
                            }
                        },
                    };
                }
            }
            b'l' => {
                let mut list = Vec::<Value>::new();
                loop {
                    match parse_bencode(reader)? {
                        None => return Ok(Some(Value::List(list))),
                        Some(v) => list.push(v),
                    }
                }
            }
            b'e' => Ok(None),
            b'0' => {
                let _ = reader.read_until(b':', &mut buf)?;
                Ok(Some(Value::Str("".to_string())))
            }
            b'1'..=b'9' => match reader.read_until(b':', &mut buf) {
                Ok(_) => {
                    let cnt = usize::from_str(&String::from_utf8_lossy(&buf[0..buf.len() - 1]))?;
                    buf.resize(cnt, 0);
                    reader.read_exact(&mut buf[0..cnt])?;
                    Ok(Some(Value::Str(
                        String::from_utf8_lossy(&buf[..]).to_string(),
                    )))
                }
                Err(e) => Err(BencodeError::Error(format!(
                    "failed to read until ':': {e}"
                ))),
            },
            x => Err(BencodeError::Error(format!("invalid character: '{x}'"))),
        },
        Err(e) => match e.kind() {
            std::io::ErrorKind::UnexpectedEof => Err(BencodeError::Eof()),
            _ => Err(BencodeError::Io(e)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_parse_bencode_num() {
        let left = vec![
            Value::Int(1),
            Value::Int(10),
            Value::Int(100_000),
            Value::Int(-1),
            Value::Int(-999),
        ];
        let right = vec!["i1e", "i10e", "i100000e", "i-1e", "i-999e"];

        for i in 0..left.len() {
            let mut bufread = BufReader::new(right[i].as_bytes());
            assert_eq!(left[i], parse_bencode(&mut bufread).unwrap().unwrap());
            assert_eq!(left[i].to_bencode(), right[i]);
        }
    }

    #[test]
    fn test_parse_bencode_str() {
        let left = vec![
            Value::Str("foo".to_string()),
            Value::Str("1234567890\n".to_string()),
            Value::Str("".to_string()),
        ];
        let right = vec!["3:foo", "11:1234567890\n", "0:"];
        for i in 0..left.len() {
            let mut bufread = BufReader::new(right[i].as_bytes());
            assert_eq!(left[i], parse_bencode(&mut bufread).unwrap().unwrap());
            assert_eq!(left[i].to_bencode(), right[i]);
        }
    }

    #[test]
    fn test_parse_bencode_list() {
        let left = vec![
            (Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
            (Value::List(vec![
                Value::Int(1),
                Value::Str("foo".to_string()),
                Value::Int(3),
            ])),
            (Value::List(vec![Value::Str("".to_string())])),
        ];
        let right = vec!["li1ei2ei3ee", "li1e3:fooi3ee", "l0:e"];
        for i in 0..left.len() {
            let mut bufread = BufReader::new(right[i].as_bytes());
            assert_eq!(left[i], parse_bencode(&mut bufread).unwrap().unwrap());
            assert_eq!(left[i].to_bencode(), right[i]);
        }
    }

    #[test]
    fn test_parse_bencode_map() {
        let mut m1 = HashMap::new();
        m1.insert(Value::Str("bar".to_string()), Value::Str("baz".to_string()));
        let m1_c = m1.clone();
        let left1 = Value::Map(m1);

        let mut m2 = HashMap::new();
        m2.insert(Value::Str("foo".to_string()), Value::Map(m1_c));
        let left2 = Value::Map(m2);

        let sright1 = "d3:bar3:baze".to_string();
        let mut right1 = BufReader::new(sright1.as_bytes());
        assert_eq!(left1, parse_bencode(&mut right1).unwrap().unwrap());
        assert_eq!(left1.to_bencode(), sright1);

        let sright2 = "d3:food3:bar3:bazee".to_string();
        let mut right2 = BufReader::new(sright2.as_bytes());
        assert_eq!(left2, parse_bencode(&mut right2).unwrap().unwrap());
        assert_eq!(left2.to_bencode(), sright2);
    }

    #[test]
    fn test_parse_bencode_map2() {
        let mut map = HashMap::new();
        map.insert(
            Value::Str("code".to_string()),
            Value::Str("(+ 1 2)\n".to_string()),
        );
        map.insert(Value::Str("op".to_string()), Value::Str("eval".to_string()));

        let input = "d4:code8:(+ 1 2)\n2:op4:evale".to_string();
        let mut reader = BufReader::new(input.as_bytes());
        assert_eq!(
            Value::Map(map),
            parse_bencode(&mut reader).unwrap().unwrap()
        );
    }
}
