#![feature(io)]
use std::io::{Read,Error};

use std::string::String;
use std::str::StrExt;

enum DataType {
    UNDEFINED,
    FLOATLE,
}

enum FieldType {
    UNDEFINED,
    UNIFORM,
}

pub struct FLDArray<'a> {
    ndim: usize,
    sizes: Vec<usize>,
    data_type: DataType,
    field_type: FieldType,
    reader: Box<Read + 'a>
}

impl<'a> FLDArray<'a> {
}

pub fn read<'a, T: Read + 'a>(mut f: T) -> Result<FLDArray<'a>, Error> {
    // stuff
    let mut ndim: usize = -1;
    let mut sizes = Vec::<usize>::new();
    let mut data_type = DataType::UNDEFINED;
    let mut field_type = FieldType::UNDEFINED;

    // read header
    let mut line = String::new();
    let mut last_char: u8 = 0;
    let mut extern_path_str = String::new();
    loop {
        let mut char_buf: [u8; 1] = [ 0u8 ];

        match f.read(&mut char_buf) {
            Ok(_) => {},
            Err(e) => return Err(e)
        };
        let c = char_buf[0];

        match (last_char, char_buf[0]) {
            (12, 12) => {
                // two form-feed characters for end of header
                break
            },
            (_, 10) => { 
                // parse line
                {
                    let words: Vec<&str> = line
                        .split('=')
                        .map(|s| s.trim())
                        .collect();
                    match &words[] {
                        ["ndim", ndim_str] => {
                            match (ndim_str.parse().ok(), ndim) {
                                (Some(i), -1) => {
                                    ndim = i;
                                    for _ in 0..ndim {
                                        sizes.push(0);
                                    }
                                }
                                (None, _) | (Some(_), _) => {
                                    panic!("ndim already set?");
                                }
                            };
                        },
                        ["dim1", dim1_str] => {
                            match (dim1_str.parse().ok(), ndim) {
                                (_, -1) => {
                                    panic!("ndim not set yet?");
                                }
                                (Some(i), _) => {
                                    sizes[0] = i;
                                }
                                (None, _) => { 
                                    panic!("dimension parse error");
                                }
                            }
                        },
                        ["dim2", dim2_str] => {
                            match (dim2_str.parse().ok(), ndim) {
                                (_, -1) => {
                                    panic!("ndim not set yet?");
                                }
                                (Some(i), _) => {
                                    sizes[1] = i;
                                }
                                (None, _) => {
                                    panic!("dimension parse error");
                                }
                            }
                        },
                        ["dim3", dim3_str] => {
                            match (dim3_str.parse().ok(), ndim) {
                                (_, -1) => {
                                    panic!("ndim not set yet?");
                                }
                                (Some(i), _) => {
                                    sizes[2] = i;
                                }
                                (None, _) => {
                                    panic!("dimension parse error");
                                }
                            }
                        },
                        ["dim4", dim4_str] => {
                            match (dim4_str.parse().ok(), ndim) {
                                (_, -1) => {
                                    panic!("ndim not set yet?");
                                }
                                (Some(i), _) => {
                                    sizes[3] = i;
                                }
                                (None, _) => {
                                    panic!("dimension parse error");
                                }
                            }
                        },
                        ["data", data_str] => {
                            data_type = 
                                match data_str {
                                    "float_le" => DataType::FLOATLE,
                                    _ => DataType::UNDEFINED
                                };
                        },
                        ["field", field_str] => {
                            field_type = 
                                match field_str {
                                    "uniform" => FieldType::UNIFORM,
                                    _ => FieldType::UNDEFINED
                                };
                        }
                        ["variable 1 file", path] => {
                            extern_path_str = path.to_string();
                        }
                        _ => {
                            println!("unhandled FLD header: {}", line);
                        }
                    }
                }
            
                // new line
                line.clear();
            }
            (_, _) => { 
                // add character to line
                line.push(c as char);
            }
        };
        last_char = char_buf[0];
    }

    // ensure field and data types are set
    match data_type {
        DataType::UNDEFINED => panic!("unknown data type"),
        _ => {}
    }
    match field_type {
        FieldType::UNDEFINED => panic!("unknown field type"),
        _ => {}
    }

    let mut tr: FLDArray<'a>;
    if extern_path_str == ""  {
        tr = FLDArray { 
            ndim: ndim,
            sizes: sizes,
            data_type: data_type,
            field_type: field_type,
            reader: Box::<T>::new(f)
        };
    } else {
        tr = FLDArray {
            ndim: ndim,
            sizes: sizes,
            data_type: data_type,
            field_type: field_type,
            reader: Box::<T>::new(f)
        };
        panic!("only one-file FLDs supported");
    }

    Ok(tr)
}

