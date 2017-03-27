use std::io::BufReader;
use std::fs::File;
use std::io::Read;
use std::string::String;
use std::vec::Vec;
use std::env;
use std::io::BufRead;
use std::io;

use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
enum BSQError {
    IOError(io::Error),
    RecvError(mpsc::RecvError),
    SendError(mpsc::SendError<Vec<u8>>),
    InvalidHeader,
    InvalidChar,
    InvalidCharFirstLine,
    InvalidEndl,
    EmptyLine,
    TrailingChars,
    MapFull,
}

#[derive(Default, Clone, Copy, Debug)]
struct Solution {
    y:usize,
    x:usize,
    score:usize,
}

#[derive(Default, Clone, Copy, Debug)]
struct Info {
    bsq_width:usize,
    bsq_height:usize,
    char_empty:u8,
    char_full:u8,
}

//Allows for the ?; syntax on IOError producing functions
impl From<io::Error> for BSQError {
    fn from(e:io::Error) -> Self {
        BSQError::IOError(e)
    }
}
//Allows for the ?; syntax on RecvError producing functions
impl From<mpsc::RecvError> for BSQError {
    fn from(e:mpsc::RecvError) -> Self {
        BSQError::RecvError(e)
    }
}

//Allows for the ?; syntax on SendError producing functions
impl From<mpsc::SendError<Vec<u8>>> for BSQError {
    fn from(e:mpsc::SendError<Vec<u8>>) -> Self {
        BSQError::SendError(e)
    }
}

impl Solution {
    pub fn new(y:usize, x:usize, score:usize) -> Solution {
        Solution {
            y: y - (score - 1),
            x: x - (score - 1),
            score: score,
        }
    }
}

//Read a line as a Vec<u8>, used to read header and get line width
fn read_line(buf:&mut BufReader<File>) -> Result<Vec<u8>, BSQError> {
    let mut s = String::new();
    buf.read_line(&mut s)?;
    Ok(s.into_bytes())
}

fn assert_error<E>(r:bool, e:E) -> Result<(), E> {
    if r {
        return Ok(());
    }
    Err(e)
}

fn check_eof(buf:&mut BufReader<File>) -> Result<(), BSQError> {

    let mut data = vec!(0; 0);

    match buf.read(&mut data) {
        Ok(0) => return Ok(()),
        _ => return Err(BSQError::TrailingChars)
    }
}

fn read_header(mut buf:&mut BufReader<File>) -> Result<(Info, Vec<u8>), BSQError> {
    let data = read_line(&mut buf)?;

    //Parses the number, and returns number and characters used to represent it
    let (val, len) = data
        .iter()
        .take_while(|&&c| c >= b'0' && c <= b'9')
        .fold((0, 0),
            |(acc, count), &c|
            (acc * 10 + (c - b'0') as usize, count + 1)
        );

    //Check the value and length of height header
    assert_error(val > 0 && len == data.len() - 4, BSQError::InvalidHeader)?;

    //Get characters
    let mut info = Info::default();

    info.bsq_height = val;
    info.char_empty = data[len];
    info.char_full = data[len + 1];
    //char_display = data[len + 2]

    assert_error(data[len + 3] == b'\n', BSQError::InvalidEndl)?;

    //Read first line
    let line = read_line(&mut buf)?;

    assert_error(line.len() > 1, BSQError::EmptyLine)?;

    info.bsq_width = line.len() - 1;

    Ok((info, line))
}

fn min3(a:usize, b:usize, c:usize) -> usize {
    let mut r = a;
    if b < r {
        r = b;
    }
    if c < r {
        r = c;
    }
    return r;
}

//Solve one bsq file, and return result or BSQError
fn scan(file_name:String) -> Result<Solution, BSQError> {

    //Open file
    let mut buf = BufReader::new(File::open(file_name)?);

    //Read header and first line
    let (info, data) = read_header(&mut buf)?;

    //Stores the current best solution
    let mut best_sqr:Solution = Solution::default();

    //Base of the walking line algorithm
    let mut current:Vec<usize> = vec![0; info.bsq_width];

    //Scan first line
    for x in 0..info.bsq_width {
        if data[x] == info.char_empty {
            current[x] = 1;
            if best_sqr.score == 0 {
                best_sqr = Solution::new(0, x, 1);
            }
        } else if data[x] == info.char_full {
            current[x] = 0;
        } else {
            println!("{:?}", data[x]);
            return Err(BSQError::InvalidCharFirstLine);
        }
    }
    assert_error(data[info.bsq_width] == b'\n', BSQError::InvalidEndl)?;


    //Setup and launch the io thread
    let (to_main, from_thread): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let (to_thread, from_main): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    thread::spawn(move || {
        for _ in 1..info.bsq_height {
            let mut data = from_main.recv().unwrap();
            buf.read_exact(&mut data).unwrap();
            to_main.send(data).unwrap();
        }
        check_eof(&mut buf).unwrap();
    });

    to_thread.send(data)?;

    //Read all lines
    for y in 1..info.bsq_height {
        let mut sc = 0;
        let mut prev_up = 0;
        let data = from_thread.recv()?;
        for x in 0..info.bsq_width {

            //if x >= read {
            //    read += buf.read(&mut data[read..])?;
            //}

            if data[x] == info.char_empty {
                //sc is previous score (x - 1), up is square just above (y - 1), prev_up is (x - 1, y - 1)
                let up = current[x];
                sc = min3(sc, prev_up, up) + 1;
                current[x] = sc;
                prev_up = up;
                if sc > best_sqr.score {
                    best_sqr =  Solution::new(y, x, sc);
                }
            } else if data[x] == info.char_full {
                prev_up = current[x];
                current[x] = 0;
                sc = 0;
            } else {
                return Err(BSQError::InvalidChar);
            }
        }
        assert_error(data[info.bsq_width] == b'\n', BSQError::InvalidEndl)?;
        to_thread.send(data)?;
    }

    //check_eof(&mut buf)?; TODO check eof

    //Return value
    if best_sqr.score == 0 {
        return Err(BSQError::MapFull);
    }
    Ok(best_sqr)
}

fn main() {
    let args = env::args().skip(1);

    for argument in args {
        match scan(argument) {
            Ok(s) => println!("{} {} {}", s.y, s.x, s.score),
            Err(e) => println!("map error {:?}", e)
        }
    }
}