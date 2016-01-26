use std::fs::{File};
use std::string::String;
use std::path::Path;
use std::convert::{From, AsRef};
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

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Error {
        Error::IO
    }
}

impl From<ParseIntError> for Error {
    fn from(_: ParseIntError) -> Error {
        Error::Parse
    }
}

#[derive(Debug)]
pub enum DataType {
    XDRFloat,
    FloatLE,
    Byte,
}

impl DataType {
    fn from_str(s: &str) -> Result<DataType, Error> {
        match s {
            "float_le" => Ok(DataType::FloatLE),
            "xdr_float" => Ok(DataType::XDRFloat),
            "byte" => Ok(DataType::Byte),
            _ => Err(Error::DataType)
        }
    }

    fn num_bytes(self: &Self) -> usize {
        match *self {
            DataType::XDRFloat => 4usize,
            DataType::FloatLE => 4usize,
            DataType::Byte => 1usize,
        }
    }

    fn convert_to_f32(self: &Self, buf: &[u8]) -> f32 {
        match *self {
            DataType::XDRFloat => {
                let b = [buf[3], buf[2], buf[1], buf[0]];
                unsafe { mem::transmute(b) }
            },
            DataType::FloatLE => {
                let b = [buf[0], buf[1], buf[2], buf[3]];
                unsafe { mem::transmute(b) }
            },
            DataType::Byte => {
                buf[0] as f32
            },
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

    pub fn read_to_f32(self: &mut Self) -> Result<Vec<f32>, Error> {
        println!("{:?}", self.sizes);
        let size = self.sizes.iter().fold(1 as usize, |l, r| l * *r);
        let mut buf_u8 = Vec::<u8>::with_capacity(size * self.data_type.num_bytes());
        let mut buf_tr = Vec::<f32>::with_capacity(size);
        try!(self.reader.read_to_end(&mut buf_u8));

        for n in 0 .. size {
            let off0 = n*self.data_type.num_bytes();
            let off1 = (n+1)*self.data_type.num_bytes();
            buf_tr.push(self.data_type.convert_to_f32(&buf_u8[off0 .. off1]));
        }

        Ok(buf_tr)
    }

    pub fn read<T>(self: &mut Self) -> Result<Vec<T>, Error> {
        let size = self.sizes.iter().fold(1 as usize, |l, r| l * *r);
        let mut buf_u8 = Vec::<u8>::with_capacity(mem::size_of::<T>()*size);
        try!(self.reader.read_to_end(&mut buf_u8));
        let buf: Vec<T> = unsafe {
            let ptr = buf_u8.as_mut_ptr();
            let cap = buf_u8.capacity();
            Vec::<T>::from_raw_parts(
                mem::transmute(ptr),
                size,
                cap / mem::size_of::<T>())
        };
        Ok(buf)
    }

    pub fn open<P: AsRef<Path>>(p: &P) -> Result<AVSFile, Error> {
        let path = p.as_ref();
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
                match tokens[0] {
                    "ndim" => {
                        let nd = try!(tokens[1].parse::<usize>());
                        ndim = Some(nd);
                        for _ in 0..nd {
                            sizes.push(None);
                        }
                    },
                    "dim1" => sizes[0] = Some(try!(tokens[1].parse::<usize>())),
                    "dim2" => sizes[1] = Some(try!(tokens[1].parse::<usize>())),
                    "dim3" => sizes[2] = Some(try!(tokens[1].parse::<usize>())),
                    "dim4" => sizes[3] = Some(try!(tokens[1].parse::<usize>())),
                    "dim5" => sizes[4] = Some(try!(tokens[1].parse::<usize>())),
                    "dim6" => sizes[5] = Some(try!(tokens[1].parse::<usize>())),
                    "dim7" => sizes[6] = Some(try!(tokens[1].parse::<usize>())),
                    "data" => 
                        data_type = Some(try!(DataType::from_str(tokens[1]))),
                    "field" => 
                        field_type = Some(try!(FieldType::from_str(tokens[1]))),
                    "variable 1 file" => 
                        external = Some(tokens[1].to_string()),
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

