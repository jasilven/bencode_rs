# bencode_rs 
Simple and minimal bencode parser.

Example program that reads bencode from stdin and prints to stdout: 

```
use bencode_rs;
use std::io::{self, BufReader};

fn main() {
    let mut reader = BufReader::new(io::stdin());

    match bencode_rs::parse_bencode(&mut reader) {
        Ok(Some(val)) => println!("{}", val.to_string()),
        Ok(None) => (),
        Err(e) => panic!("Error: {} ", e),
    }
}
```
