# bencode_rs 
Simple and minimal bencode parser and encoder.

Example program that reads bencode from stdin and prints to stdout: 

```
[dependencies]
bencode_rs = { git = "https://github.com/jasilven/bencode_rs", tag = "v0.1.0" }
```

```
use bencode_rs::parse_bencode;
use std::io::{self, BufReader};

fn main() {
    let mut reader = BufReader::new(io::stdin());

    match parse_bencode(&mut reader) {
        Ok(Some(val)) => {
            println!("Parsed string {}", val.to_string()),
            println!("Generate bencode: {}", val.to_bencode());
        }
        Ok(None) => (),
        Err(e) => panic!("Error: {} ", e),
    }
}
```
