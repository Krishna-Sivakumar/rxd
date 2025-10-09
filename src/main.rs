pub mod argparse;
pub mod format;
use crate::format::{Color, swap_nibbles, to_binary, to_lower_hex, to_upper_hex};
use std::{cmp::min, env, fs};

const HELP_TEXT: &str = "
USAGE: rxd [options] [[infile] [outfile]]
";

const VERSION: &str = "0.1";

fn include_format(
    inhandle: &mut Box<dyn std::io::Read>,
    outhandle: &mut Box<dyn std::io::Write>,
    options: &argparse::Options,
) {
    let columns = options.cols.unwrap_or(30);
    let mut buf: Vec<u8> = Vec::with_capacity(columns * 128);
    let include_filename = options.include_name.clone().unwrap_or("buffer".into());

    let mut line_buf: String = String::new();
    let mut tot_bytes = 0;
    line_buf.push_str(format!("unsigned char {}[] = {{\n", include_filename).as_str());
    while let Ok(n_bytes) = inhandle.read(&mut buf) {
        tot_bytes += n_bytes;
        for row in 0..(n_bytes / columns + 1) {
            if row * columns == n_bytes {
                continue;
            }

            line_buf.push_str("  ");
            for byte in buf[row * columns..row * columns].as_ref().into_iter() {
                let outbyte = swap_nibbles(*byte);
                std::fmt::write(
                    &mut line_buf,
                    format_args!("0x{:x}{:x}, ", outbyte & 15, outbyte >> 4 & 15),
                )
                .expect("write must succeed.");
            }
            line_buf.push('\n');
        }
    }

    line_buf.push_str(
        format!(
            "}};\nunsigned int {} = {};",
            include_filename + "_len",
            tot_bytes
        )
        .as_str(),
    );

    outhandle
        .write_all(line_buf.as_bytes())
        .expect("Could not write to handle.");
}

fn main() {
    use argparse::Options;

    let arguments: Vec<String> = env::args().collect(); //aaaa
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

    // this is the source of the memory leak
    // TODO read slowly from the source
    let mut inhandle: Box<dyn std::io::Read> = match options.infile {
        Some(ref filename) => match fs::File::open(&filename) {
            Err(err) => {
                println!("Could not open {}: {}", &filename, err.to_string());
                return;
            }
            Ok(handle) => Box::new(handle),
        },
        None => Box::new(std::io::stdin()),
    };

    let mut outhandle: Box<dyn std::io::Write> = match options.outfile {
        None => Box::new(std::io::stdout()),
        Some(ref filename) => {
            Box::new(fs::File::create(filename).expect("Could not create output file."))
        }
    };

    // Doing this as branching might be a problem (if dispatch isn't...) and it's easier to manage the code here
    let formatter = if options.bits {
        to_binary
    } else if options.uppercase {
        to_upper_hex
    } else {
        to_lower_hex
    };

    // the size of contents (Vec<u8>) needs to be limited by -len_octets
    /*
    match options.len_octets {
        None => {
            inhandle
                .read_to_end(&mut contents)
                .expect("Could not read from stream.");
        }
        Some(len) => {
            contents.resize_with(len, || 0_u8);
            inhandle
                .read_exact(&mut contents)
                .expect("Could not read from stream.");
        }
    }
    */

    let mut buffer = String::new();

    let columns = options.cols.unwrap_or(if options.include_format {
        12
    } else if options.postscript_style {
        30
    } else if options.bits {
        6
    } else {
        16
    });

    if options.include_format {
        include_format(&mut inhandle, &mut outhandle, &options);
        return;
    }

    let mut total_bytes_read: usize = 0;
    let mut group_buf: Vec<u8> = Vec::new();
    group_buf.resize(columns * 128, 0);

    let mut line_hexbuf = String::new();
    let mut line_buf = String::new();
    let mut row_counter: usize = 0;

    while let Ok(bytes_read) = inhandle.read(&mut group_buf) {
        total_bytes_read += bytes_read;
        if bytes_read == 0 {
            break;
        }

        for row in 0..(bytes_read / columns + 1) {
            line_hexbuf.clear();
            line_buf.clear();

            for (idx, byte) in group_buf[row * columns..min(row * columns + columns, bytes_read)]
                .as_ref()
                .iter()
                .enumerate()
            {
                if *byte == 0 {
                    line_buf.push_str(Color::White.ansi());
                    line_hexbuf.push_str(Color::White.ansi());
                } else if *byte == 0xa || *byte == 0x9 || *byte == 0x20 {
                    line_buf.push_str(Color::Yellow.ansi());
                    line_hexbuf.push_str(Color::Yellow.ansi());
                } else if *byte == 0xff {
                    line_buf.push_str(Color::Blue.ansi());
                    line_hexbuf.push_str(Color::Blue.ansi());
                } else if byte.is_ascii_graphic() {
                    line_buf.push_str(Color::Green.ansi());
                    line_hexbuf.push_str(Color::Green.ansi());
                } else {
                    line_buf.push_str(Color::Red.ansi());
                    line_hexbuf.push_str(Color::Red.ansi());
                }

                if !byte.is_ascii_graphic() && *byte != 32 {
                    line_buf.push('.')
                } else {
                    line_buf.push((*byte).into())
                }

                if !options.postscript_style && idx % options.group_size == 0 && idx > 0 {
                    line_hexbuf.push(' ')
                }

                let outbyte = swap_nibbles(*byte);
                formatter(&mut line_hexbuf, &outbyte);
            }

            if options.postscript_style {
                buffer += line_hexbuf.as_str();
            } else if line_hexbuf.len() > 0 {
                std::fmt::write(
                    &mut buffer,
                    format_args!(
                        "{:0>8x}: {}{: <39}  {}{}",
                        row_counter * columns,
                        Color::Bold.ansi(),
                        line_hexbuf,
                        line_buf,
                        Color::Reset.ansi()
                    ),
                )
                .expect("Write must succeed.");
            }

            // if there's nothing in the slice, don't add a newline
            if min(row * columns + columns, bytes_read) - row * columns > 0 {
                buffer.push('\n');
                row_counter += 1;
            }

            outhandle
                .write_all(buffer.as_bytes())
                .expect("Could not write to handle.");
            buffer.clear();
        }
    }

    outhandle
        .write_all(buffer.trim().as_bytes())
        .expect("Could not write to handle.");
    println!(""); // final newline
}
