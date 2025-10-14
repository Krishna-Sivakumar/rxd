pub mod argparse;
pub mod bufio;
pub mod format;
use crate::format::{Color, swap_nibbles, to_binary, to_lower_hex, to_upper_hex};
use std::io::IsTerminal;
use std::{cmp::min, env, fs};

const HELP_TEXT: &str = "
USAGE: rxd [options] [[infile] [outfile]]
";

const VERSION: &str = "0.1";

/// prints bytes read from `inhandle` to `outhandle` in C include format.
fn include_format(
    inhandle: Box<dyn std::io::Read>,
    mut outhandle: Box<dyn std::io::Write>,
    options: argparse::Options,
) {
    let columns = options.cols.unwrap_or(30);
    let mut reader = bufio::LimitedBufReader::new(columns * 128, inhandle, options.len_octets);

    let mut buffer_name = options.include_name.clone().unwrap_or("buffer".into());
    let mut buffer_length_name = buffer_name.clone() + "_len";

    if options.capitalize {
        buffer_name = buffer_name.to_ascii_uppercase();
        buffer_length_name = buffer_length_name.to_uppercase();
    }

    let mut line_buf: String = String::new();
    line_buf.push_str(format!("unsigned char {}[] = {{\n", buffer_name).as_str());

    while let Ok(n_bytes) = reader.read() {
        if n_bytes == 0 {
            break;
        }

        let buf = reader.as_ref();

        for row in 0..(n_bytes / columns + 1) {
            if row * columns == n_bytes {
                continue;
            }

            line_buf.push_str("  ");
            for byte in buf[row * columns..(row * columns + columns)]
                .as_ref()
                .into_iter()
            {
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
            "}};\nunsigned int {} = {};\n",
            buffer_length_name,
            reader.total_bytes_read()
        )
        .as_str(),
    );

    outhandle
        .write_all(line_buf.as_bytes())
        .expect("Could not write to handle.");
}

/// prints bytes read from `inhandle` to `outhandle` in xxd's regular format.
fn regular_format(
    inhandle: Box<dyn std::io::Read>,
    mut outhandle: Box<dyn std::io::Write>,
    options: argparse::Options,
    is_terminal: bool,
) {
    // Doing this as branching might be a problem (if dispatch isn't...) and it's easier to manage the code here
    let formatter = if options.bits {
        to_binary
    } else if options.uppercase {
        to_upper_hex
    } else {
        to_lower_hex
    };

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

    let mut line_hexbuf = String::new();
    let mut line_buf = String::new();
    let mut row_counter: usize = 0;

    let mut reader = bufio::LimitedBufReader::new(columns * 128, inhandle, options.len_octets);

    while let Ok(bytes_read) = reader.read() {
        if bytes_read == 0 {
            break;
        }

        let bytes = reader.as_ref();

        for row in 0..(bytes_read / columns + 1) {
            line_hexbuf.clear();
            line_buf.clear();

            let mut graphic_bytes = 0; // the amount of graphic bytes written to line_hexbuf

            let slice = bytes[row * columns..min(row * columns + columns, bytes_read)].as_ref();

            if false {
                for group in 0..(columns / options.group_size) {
                    if !options.postscript_style {
                        line_hexbuf.push(' ');
                    }
                }
            }

            for (idx, byte) in slice.iter().enumerate() {
                if is_terminal {
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
                }

                if !byte.is_ascii_graphic() && *byte != 32 {
                    line_buf.push('.')
                } else {
                    line_buf.push((*byte).into())
                }

                if !options.postscript_style && idx % options.group_size == 0 && idx > 0 {
                    line_hexbuf.push(' ');
                    graphic_bytes += 1;
                }

                let outbyte = swap_nibbles(*byte);
                formatter(&mut line_hexbuf, &outbyte);

                graphic_bytes += 2;
            }

            if options.postscript_style {
                buffer += line_hexbuf.as_str();
            } else if line_hexbuf.len() > 0 {
                // padding calculation; check how many bytes line_hexbuf needs to be padded out
                // so that line_buf appears in a straight line.
                let total_width = columns * 2 + (columns / options.group_size);
                let padding = if total_width < graphic_bytes {
                    0
                } else {
                    total_width - graphic_bytes
                };

                for _ in 0..padding {
                    line_hexbuf.push(' ');
                }

                if is_terminal {
                    std::fmt::write(
                        &mut buffer,
                        format_args!(
                            "{:0>8x}: {}{}  {}{}",
                            row_counter * columns,
                            Color::Bold.ansi(),
                            line_hexbuf, // TODO extend this with `padding` length spaces.
                            line_buf,
                            Color::Reset.ansi()
                        ),
                    )
                    .expect("Write must succeed.");
                } else {
                    std::fmt::write(
                        &mut buffer,
                        format_args!(
                            "{:0>8x}: {}  {}",
                            row_counter * columns,
                            line_hexbuf,
                            line_buf,
                        ),
                    )
                    .expect("Write must succeed.");
                }
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
    let inhandle: Box<dyn std::io::Read> = match options.infile {
        Some(ref filename) => match fs::File::open(&filename) {
            Err(err) => {
                println!("Could not open {}: {}", &filename, err.to_string());
                return;
            }
            Ok(handle) => Box::new(handle),
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

    if options.include_format {
        include_format(inhandle, outhandle, options);
        return;
    }

    regular_format(inhandle, outhandle, options, is_terminal);
}
