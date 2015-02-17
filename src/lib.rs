#![feature(io)]
#![feature(path)]
#![feature(fs)]
#![feature(collections)]
#![feature(core)]

use std::fs::{File};
use std::string::String;
use std::str::StrExt;
use std::path::AsPath;
use std::error::FromError;
use std::io::{Read, BufReader};
use std::vec::Vec;
use std::num::ParseIntError;
use std::mem;

pub enum Error {
    IOError,
    ParseError,
    DataTypeError,
    FieldTypeError,
    MalformedError,
    NotImplemented
}

impl FromError<std::io::Error> for Error {
    fn from_error(_: std::io::Error) -> Error {
        Error::IOError
    }
}

impl FromError<ParseIntError> for Error {
    fn from_error(_: ParseIntError) -> Error {
        Error::ParseError
    }
}

pub enum DataType {
    XDRFloat,
    FloatLE,
}

impl DataType {
    fn from_str(s: &str) -> Result<DataType, Error> {
        match s {
            "float_le" => Ok(DataType::FloatLE),
            "xdr_float" => Ok(DataType::XDRFloat),
            _ => Err(Error::DataTypeError)
        }
    }
}

pub enum FieldType {
    Uniform
}

impl FieldType {
    fn from_str(s: &str) -> Result<FieldType, Error> {
        match s {
            "uniform" => Ok(FieldType::Uniform),
            _ => Err(Error::FieldTypeError)
        }
    }
}

pub struct AVSFile<'a> {
    pub ndim: usize,
    pub sizes: Vec<usize>,
    pub data_type: DataType,
    pub field_type: FieldType,
    reader: Box<Read + 'a>
}

impl<'a> AVSFile<'a> {
    pub fn read_f32(self: &mut Self) -> Result<f32, Error> {
        match self.data_type {
            DataType::XDRFloat => {
                let mut buf = [ 0u8; 4 ];
                assert!(try!(self.reader.read(&mut buf)) == 4);
                (&mut buf).reverse();
                Ok(unsafe { mem::transmute(buf) } )
            }
            DataType::FloatLE => {
                let mut buf = [ 0u8; 4 ];
                assert!(try!(self.reader.read(&mut buf)) == 4);
                Ok(unsafe { mem::transmute(buf) } )
            }
        }
    }

    pub fn open<P: AsPath>(path: &P) -> Result<AVSFile, Error> {
        let mut reader = BufReader::new(try!(File::open(path)));

        let mut ndim: Option<usize> = None;
        let mut sizes = Vec::<Option<usize>>::new();
        let mut data_type: Option<DataType> = None;
        let mut field_type: Option<FieldType> = None;
        let mut external: Option<String> = None;

        let mut line = String::new();
        let mut last_char: u8 = 0;
        loop {
            let mut new_char_buf: [u8;1] = [ 0u8 ];
            try!(reader.read(&mut new_char_buf));

            // break on two chr 14s
            let new_char = new_char_buf[0];
            if (new_char, last_char) == (14u8, 14u8) {
                break;
            }
            last_char = new_char;

            line.push(new_char as char);

            // new line; process the line and discard
            if new_char == 10 {
                let tokens: Vec<&str> = line.split('=')
                    .map(|s| s.trim()).collect();
                match &tokens[] {
                    ["ndim", s] => {
                        let nd = try!(s.parse::<usize>());
                        ndim = Some(nd);
                        for _ in 0..nd {
                            sizes.push(None);
                        }
                    },
                    ["dim1", s] => sizes[0] = Some(try!(s.parse::<usize>())),
                    ["dim2", s] => sizes[1] = Some(try!(s.parse::<usize>())),
                    ["dim3", s] => sizes[2] = Some(try!(s.parse::<usize>())),
                    ["dim4", s] => sizes[2] = Some(try!(s.parse::<usize>())),
                    ["dim5", s] => sizes[2] = Some(try!(s.parse::<usize>())),
                    ["dim6", s] => sizes[2] = Some(try!(s.parse::<usize>())),
                    ["dim7", s] => sizes[2] = Some(try!(s.parse::<usize>())),
                    ["data", s] => 
                        data_type = Some(try!(DataType::from_str(s))),
                    ["field", s] => 
                        field_type = Some(try!(FieldType::from_str(s))),
                    ["variable 1 file", s] => 
                        external = Some(String::from_str(s)),
                    _ => {}
                }
            }
            // hack?  code smell?  need borrow in previous block to expire
            if new_char == 10 {
                line.clear();
            }
        }

        match external {
            None => {
                let mut tr = AVSFile { 
                    ndim: try!(ndim.ok_or(Error::MalformedError)),
                    sizes: Vec::<usize>::new(),
                    data_type: try!(data_type.ok_or(Error::MalformedError)),
                    field_type: try!(field_type.ok_or(Error::MalformedError)),
                    reader: Box::new(reader),
                };
                for idx in 0..ndim.unwrap() {
                    tr.sizes.push(
                        try!(sizes[idx].ok_or(Error::MalformedError)));
                }
                Ok(tr)
            },
            Some(path) => {
                let new_reader = BufReader::new(try!(File::open(&path)));
                let mut tr = AVSFile { 
                    ndim: try!(ndim.ok_or(Error::MalformedError)),
                    sizes: Vec::<usize>::new(),
                    data_type: try!(data_type.ok_or(Error::MalformedError)),
                    field_type: try!(field_type.ok_or(Error::MalformedError)),
                    reader: Box::new(new_reader),
                };
                for idx in 0..ndim.unwrap() {
                    tr.sizes.push(
                        try!(sizes[idx].ok_or(Error::MalformedError)));
                }
                Ok(tr)
            },
        }
    }
}

