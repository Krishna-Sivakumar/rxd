pub mod argparse;
pub mod bufio;
pub mod format;
pub mod revert;
use crate::format::{Color, to_binary, to_lower_hex, to_upper_hex};
use std::io::{IsTerminal, Seek, SeekFrom, Write};
use std::{env, fs};

#[derive(Debug)]
pub enum RxdError {
    Message(String),
    IoError(std::io::Error),
}

impl From<std::io::Error> for RxdError {
    fn from(err: std::io::Error) -> Self {
        RxdError::IoError(err)
    }
}

impl std::fmt::Display for RxdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RxdError::Message(s) => write!(f, "Error: {s}"),
            RxdError::IoError(e) => write!(f, "Error: {e}"),
        }
    }
}

impl std::error::Error for RxdError {}

const HELP_TEXT: &str = "
Usage:
       xxd [options] [infile [outfile]]
    or
       xxd -r [-s [-]offset] [-c cols] [-ps] [infile [outfile]]
Options:
    -a          toggle autoskip: A single '*' replaces nul-lines. Default off.
    -b          binary digit dump (incompatible with -ps,-i). Default hex.
    -C          capitalize variable names in C include file style (-i).
    -c cols     format <cols> octets per line. Default 16 (-i: 12, -ps: 30).
    -e          little-endian dump (incompatible with -ps,-i,-r).
    -g bytes    number of octets per group in normal output. Default 2 (-e: 4).
    -h          print this summary.
    -i          output in C include file style.
    -l len      stop after <len> octets.
    -n name     set the variable name used in C include output (-i).
    -o off      add <off> to the displayed file position.
    -ps         output in postscript plain hexdump style.
    -r          reverse operation: convert (or patch) hexdump into binary.
    -r -s off   revert with <off> added to file positions found in hexdump.
    -d          show offset in decimal instead of hex.
    -s [+][-]seek  start at <seek> bytes abs. (or +: rel.) infile offset.
    -u          use upper case hex letters.
    -R when     colorize the output; <when> can be 'always', 'auto' or 'never'. Default: 'auto'.
    -v          show version: \"rxd 2025-10 by Krishna Sivakumar\".
";

const VERSION: &str = "rxd 2025-10 by Krishna Sivakumar";

/// prints bytes read from `inhandle` to `outhandle` in C include format.
fn include_format(
    inhandle: Box<dyn std::io::Read>,
    outhandle: Box<dyn std::io::Write>,
    options: argparse::Options,
) -> Result<(), RxdError> {
    let columns = options.cols.unwrap_or(30);
    let mut reader = bufio::LimitedBufReader::new(columns * 128, inhandle, options.len_octets);

    let mut buffer_name = options.include_name.clone().unwrap_or("buffer".into());
    let mut buffer_length_name = buffer_name.clone() + "_len";

    if options.capitalize {
        buffer_name = buffer_name.to_ascii_uppercase();
        buffer_length_name = buffer_length_name.to_uppercase();
    }

    let mut outbuf = std::io::BufWriter::with_capacity(8192, outhandle);

    outbuf.write_fmt(format_args!("unsigned char {}[] = {{\n", buffer_name))?;

    while let Ok(n_bytes) = reader.read() {
        if n_bytes == 0 {
            break;
        }

        let buf = reader.as_ref();

        for row in buf.chunks(columns) {
            outbuf.write("  ".as_bytes())?;
            for byte in row.as_ref().into_iter() {
                outbuf.write("0x".as_bytes())?;
                to_lower_hex(&mut outbuf, &byte);
                outbuf.write(", ".as_bytes())?;
            }
            outbuf.write("\n".as_bytes())?;
        }
    }

    outbuf.write_fmt(format_args!(
        "}};\nunsigned int {} = {};\n",
        buffer_length_name,
        reader.total_bytes_read()
    ))?;

    Ok(())
}

/// prints bytes read from `inhandle` to `outhandle` in postscript (only hex bytes) format.
fn postscript_format(
    mut inhandle: Box<dyn std::io::Read>,
    outhandle: Box<dyn std::io::Write>,
    options: argparse::Options,
) -> Result<(), RxdError> {
    let columns = options.cols.unwrap_or(16);

    if options.seek != 0 {
        if options.seek < 0 {
            return Err(RxdError::Message("Could not seek to location".into()));
        }
        let mut tempbuf: Vec<u8> = Vec::new();
        tempbuf.resize(options.seek.abs_diff(0) as usize, 0);
        inhandle.read(&mut tempbuf)?;
    }

    let mut reader = bufio::LimitedBufReader::new(columns * 128 * 16, inhandle, options.len_octets);
    let mut writer = std::io::BufWriter::with_capacity(columns * 128 * 16, outhandle);

    while let Ok(bytes_read) = reader.read() {
        if bytes_read == 0 {
            break;
        }

        let bytes = reader.as_ref();
        for chunk in bytes.chunks(columns) {
            for byte in chunk {
                to_lower_hex(&mut writer, byte);
            }
            writer.write("\n".as_bytes())?; // don't need to check this result
        }
    }

    Ok(())
}

/// prints bytes read from `inhandle` to `outhandle` in xxd's regular format.
fn regular_format(
    mut inhandle: Box<dyn std::io::Read>,
    outhandle: Box<dyn std::io::Write>,
    options: argparse::Options,
    is_terminal: bool,
) -> Result<(), RxdError> {
    // Doing this as branching might be a problem (if dispatch isn't...) and it's easier to manage the code here
    let formatter = if options.bits {
        to_binary
    } else if options.uppercase {
        to_upper_hex
    } else {
        to_lower_hex
    };

    let columns = options.cols.unwrap_or(if options.include_format {
        12
    } else if options.postscript_style {
        30
    } else if options.bits {
        6
    } else {
        16
    });

    let mut row_counter: usize = 0;

    if options.seek != 0 {
        if options.seek < 0 {
            return Err(RxdError::Message("Sorry, cannot seek.".to_owned()));
        }
        let mut tempbuf: Vec<u8> = Vec::new();
        tempbuf.resize(options.seek.abs_diff(0) as usize, 0);
        inhandle.read(&mut tempbuf)?;
    }

    let mut reader = bufio::LimitedBufReader::new(columns * 128 * 16, inhandle, options.len_octets);
    let mut buffer = std::io::BufWriter::with_capacity(8192, outhandle);

    while let Ok(bytes_read) = reader.read() {
        if bytes_read == 0 {
            break;
        }

        let bytes = reader.as_ref();

        for slice in bytes.get(0..bytes_read).unwrap().chunks(columns) {
            let mut graphic_bytes = 0; // the amount of graphic bytes written to line_hexbuf

            fn get_colour(byte: &u8) -> Color {
                if *byte == 0 {
                    Color::White
                } else if *byte == 0xa || *byte == 0x9 || *byte == 0x20 {
                    Color::Yellow
                } else if *byte == 0xff {
                    Color::Blue
                } else if byte.is_ascii_graphic() {
                    Color::Green
                } else {
                    Color::Red
                }
            }

            buffer.write_fmt(format_args!("{:0>8x}: ", row_counter * columns))?;
            if is_terminal {
                buffer.write(Color::Bold.ansi().as_bytes())?;
            }

            for group in slice.chunks(options.group_size) {
                if options.is_little_endian {
                    for byte in group.iter().rev() {
                        if is_terminal {
                            let colour = get_colour(byte);
                            buffer.write(colour.ansi().as_bytes())?;
                        }
                        formatter(&mut buffer, &byte);
                        graphic_bytes += 2;
                    }
                } else {
                    for byte in group.iter() {
                        if is_terminal {
                            let colour = get_colour(byte);
                            buffer.write(colour.ansi().as_bytes())?;
                        }
                        formatter(&mut buffer, &byte);
                        graphic_bytes += 2;
                    }
                }

                buffer.write(" ".as_bytes())?;
            }

            buffer.write(" ".as_bytes())?;

            for group in slice.chunks(options.group_size) {
                for byte in group {
                    if is_terminal {
                        let colour = get_colour(byte);
                        buffer.write(colour.ansi().as_bytes())?;
                    }

                    if !byte.is_ascii_graphic() && *byte != 0x20 {
                        buffer.write(".".as_bytes())?;
                    } else {
                        buffer.write(&[*byte])?;
                    }
                }
            }

            // padding calculation; check how many bytes line_hexbuf needs to be padded out
            // so that line_buf appears in a straight line.
            let total_width = columns * 2 + (columns / options.group_size);
            let padding = if total_width < graphic_bytes {
                0
            } else {
                total_width - graphic_bytes
            };

            for _ in 0..padding {
                buffer.write(" ".as_bytes())?;
            }

            buffer.write(Color::Reset.ansi().as_bytes())?;

            buffer.write("\n".as_bytes())?;

            row_counter += 1;
        }
    }

    Ok(())
}

fn main() {
    use argparse::Options;

    let arguments: Vec<String> = env::args().collect();
    let options = match Options::parse_options(arguments[1..].to_owned()) {
        Ok(opt) => opt,
        Err(err) => {
            println!("{}", err);
            println!("{}", HELP_TEXT);
            return;
        }
    };

    if options.display_help {
        println!("{}", HELP_TEXT);
        return;
    }

    if options.display_version {
        println!("{}", VERSION);
        return;
    }

    let inhandle: Box<dyn std::io::Read> = match options.infile {
        Some(ref filename) => match fs::File::open(&filename) {
            Err(err) => {
                println!("Could not open {}: {}", &filename, err.to_string());
                return;
            }
            Ok(mut handle) => {
                if options.seek > 0 {
                    handle
                        .seek(SeekFrom::Start(options.seek.abs_diff(0).into()))
                        .expect("Could not seek to location.");
                } else if options.seek < 0 {
                    handle
                        .seek(SeekFrom::End(options.seek.into()))
                        .expect("Could not seek to location.");
                }
                Box::new(handle)
            }
        },
        None => Box::new(std::io::stdin()),
    };

    let (outhandle, is_terminal): (Box<dyn std::io::Write>, bool) = match options.outfile {
        None => {
            let stdout = std::io::stdout();
            let is_terminal = stdout.is_terminal();
            (Box::new(stdout), is_terminal)
        }
        Some(ref filename) => {
            let file = fs::File::create(filename).expect("Could not create output file.");
            let is_terminal = file.is_terminal();

            (Box::new(file), is_terminal)
        }
    };

    if options.revert {
        if let Err(e) = revert::revert(inhandle, outhandle, options) {
            println!("{:?}", e);
        }
    } else if options.include_format {
        if let Err(e) = include_format(inhandle, outhandle, options) {
            println!("{:?}", e);
        }
    } else if options.postscript_style {
        if let Err(e) = postscript_format(inhandle, outhandle, options) {
            println!("{:?}", e);
        }
        return;
    } else {
        if let Err(e) = regular_format(inhandle, outhandle, options, is_terminal) {
            println!("{:?}", e);
        }
        return;
    }
}
