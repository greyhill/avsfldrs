#![feature(collections)]
#![feature(slice_patterns)]

use std::fs::{File};
use std::string::String;
use std::path::Path;
use std::error::FromError;
use std::io::{Read, BufReader, Write};
use std::vec::Vec;
use std::num::ParseIntError;
use std::mem;

#[derive(Debug)]
pub enum Error {
    IO,
    Parse,
    DataType,
    FieldType,
    Malformed,
    NotImplemented
}

impl FromError<std::io::Error> for Error {
    fn from_error(_: std::io::Error) -> Error {
        Error::IO
    }
}

impl FromError<ParseIntError> for Error {
    fn from_error(_: ParseIntError) -> Error {
        Error::Parse
    }
}

#[derive(Debug)]
pub enum DataType {
    XDRFloat,
    FloatLE,
}

impl DataType {
    fn from_str(s: &str) -> Result<DataType, Error> {
        match s {
            "float_le" => Ok(DataType::FloatLE),
//            "xdr_float" => Ok(DataType::XDRFloat),
            _ => Err(Error::DataType)
        }
    }
}

#[derive(Debug)]
pub enum FieldType {
    Uniform
}

impl FieldType {
    fn from_str(s: &str) -> Result<FieldType, Error> {
        match s {
            "uniform" => Ok(FieldType::Uniform),
            _ => Err(Error::FieldType)
        }
    }
}

pub struct AVSFile {
    pub ndim: usize,
    pub sizes: Vec<usize>,
    pub data_type: DataType,
    pub field_type: FieldType,
    reader: Box<Read>
}

impl AVSFile {
    pub fn write<W: Write, T>(
                writer: &mut W, dims: &[usize], data: &[T]) 
                    -> Result<(), Error> {
        // header
        let ndim = dims.len();
        try!(writer.write_fmt(format_args!("# AVS FLD file (written by avsfldrs github.com/greyhill/avsfldrs)\n")));
        try!(writer.write_fmt(format_args!("ndim={}\n", ndim)));
        try!(writer.write_fmt(format_args!("veclen=1\n")));
        try!(writer.write_fmt(format_args!("nspace={}\n", ndim)));
        try!(writer.write_fmt(format_args!("field=uniform\n")));
        try!(writer.write_fmt(format_args!("data=float_le\n"))); // TODO
        for (id, size) in dims.iter().enumerate() {
            try!(writer.write_fmt(format_args!("dim{}={}\n", id+1, size)));
        }
        try!(writer.write_fmt(format_args!("{}{}", 12 as char, 12 as char)));
        let b: &[u8] = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const u8, 
                                       data.len()*mem::size_of::<T>())
        };
        try!(writer.write_all(b));
        Ok(())
    }

    pub fn read<T>(self: &mut Self) -> Result<Vec<T>, Error> {
        let size = self.sizes.iter().fold(1 as usize, |l, r| l * *r);
        let mut buf_u8 = Vec::<u8>::with_capacity(mem::size_of::<T>()*size);
        try!(self.reader.read_to_end(&mut buf_u8));
        let buf: Vec<T> = unsafe {
            let ptr = buf_u8.as_mut_ptr();
            let cap = buf_u8.capacity();
            mem::forget(buf_u8);
            Vec::<T>::from_raw_parts(
                mem::transmute(ptr),
                size,
                cap / mem::size_of::<T>())
        };
        Ok(buf)
    }

    pub fn open(path: &Path) -> Result<AVSFile, Error> {
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
            if (new_char, last_char) == (12u8, 12u8) {
                break;
            }
            last_char = new_char;

            line.push(new_char as char);

            // new line; process the line and discard
            if new_char == 10 {
                let tokens: Vec<&str> = line.split('=')
                    .map(|s| s.trim()).collect();
                match &tokens[..] {
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
                    ["dim4", s] => sizes[3] = Some(try!(s.parse::<usize>())),
                    ["dim5", s] => sizes[4] = Some(try!(s.parse::<usize>())),
                    ["dim6", s] => sizes[5] = Some(try!(s.parse::<usize>())),
                    ["dim7", s] => sizes[6] = Some(try!(s.parse::<usize>())),
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
                    ndim: try!(ndim.ok_or(Error::Malformed)),
                    sizes: Vec::<usize>::new(),
                    data_type: try!(data_type.ok_or(Error::Malformed)),
                    field_type: try!(field_type.ok_or(Error::Malformed)),
                    reader: Box::new(reader),
                };
                for idx in 0..ndim.unwrap() {
                    tr.sizes.push(
                        try!(sizes[idx].ok_or(Error::Malformed)));
                }
                Ok(tr)
            },
            Some(path) => {
                let new_reader = BufReader::new(try!(File::open(&path)));
                let mut tr = AVSFile { 
                    ndim: try!(ndim.ok_or(Error::Malformed)),
                    sizes: Vec::<usize>::new(),
                    data_type: try!(data_type.ok_or(Error::Malformed)),
                    field_type: try!(field_type.ok_or(Error::Malformed)),
                    reader: Box::new(new_reader),
                };
                for idx in 0..ndim.unwrap() {
                    tr.sizes.push(
                        try!(sizes[idx].ok_or(Error::Malformed)));
                }
                Ok(tr)
            },
        }
    }
}

