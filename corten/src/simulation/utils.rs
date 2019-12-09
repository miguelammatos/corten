// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0>.
// This file may not be copied, modified, or distributed
// except according to those terms.

use std::io::Write;
use std::path::Path;
use std::fs::OpenOptions;
use std::fs;
use std::fmt::{Debug, Display};

use serde::{Serialize};
use serde_yaml;

use bincode::{serialize, deserialize};

use crate::simulation::ProcessId;

pub fn save_to_file<P: AsRef<Path>, S: Into<String> + Debug + Display>(filename: P, content: S, append: bool) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(append)
        .create(true)
        .open(filename)
        .unwrap();

    writeln!(file, "{}", content).unwrap();
}

pub fn save_u8_to_file<P: AsRef<Path>>(filename: P, content: &[u8], append: bool) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(append)
        .create(true)
        .open(filename)
        .unwrap();

    file.write(content).unwrap();
}

pub fn vec_to_str_no_whitespace(v: Vec<ProcessId>) -> String {
    let mut s: String = "[".to_string();
    for i in 0..v.len()-1 {
        s += &format!("{},", v[i]);
    }
    s += &format!("{}]", v[v.len()-1]);
    s
}

pub fn vec_to_one_item_per_line<T: Display>(v: &Vec<T>) -> String {
    let mut s: String = "".to_string();
    for item in v {
        s += &format!("{}\n", item);
    }
    s
}

pub fn yaml_from_file_to_object<P: AsRef<Path> + Display, O: serde::de::DeserializeOwned + Debug>(filename: P) -> O {
    let s = fs::read_to_string(&filename).expect(&format!("Unable to open and read from file {}", &filename));

    let c = serde_yaml::from_str(&s).unwrap();

    c
}

pub fn save_object_in_yaml_file<O: Serialize, P: AsRef<Path>>(obj: &O, filename: P) {
    let serialized = serde_yaml::to_string(&obj).unwrap();

    save_to_file(filename, serialized, false);
}

pub fn binary_from_file_to_object<P: AsRef<Path> + Display, O: serde::de::DeserializeOwned + Debug>(filename: P) -> O {
    let s = fs::read(&filename).expect(&format!("Unable to open and read from file {}", &filename));

    let decoded: O = deserialize(&s[..]).unwrap();

    decoded
}

pub fn save_object_in_binary_file<O: Serialize, P: AsRef<Path>>(obj: &O, filename: P) {
    let encoded: Vec<u8> = serialize(&obj).unwrap();

    save_u8_to_file(filename, &encoded, false);
}
