pub mod argparse;
pub mod bufio;
pub mod format;
use crate::format::{Color, to_binary, to_lower_hex, to_upper_hex};
use std::io::{IsTerminal, Seek, SeekFrom};
use std::{env, fs};

// TODO copy xxd help text here at some point
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
                line_buf.push_str("0x");
                format::to_lower_hex(&mut line_buf, &byte);
                line_buf.push_str(", ");
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
    mut inhandle: Box<dyn std::io::Read>,
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

    let columns = options.cols.unwrap_or(if options.include_format {
        12
    } else if options.postscript_style {
        30
    } else if options.bits {
        6
    } else {
        16
    });

    let mut buffer = String::with_capacity(columns * 128 * 4);
    let mut line_hexbuf = String::with_capacity(columns * 128 * 4);
    let mut line_buf = String::with_capacity(columns * 128 * 4);
    let mut row_counter: usize = 0;

    if options.seek != 0 {
        if options.seek < 0 {
            println!("Sorry, cannot seek.");
            return;
        }
        let mut tempbuf: Vec<u8> = Vec::new();
        tempbuf.resize(options.seek.abs_diff(0) as usize, 0);
        inhandle
            .read(&mut tempbuf)
            .expect("Could not seek to location.");
    }

    let mut reader = bufio::LimitedBufReader::new(columns * 128, inhandle, options.len_octets);

    while let Ok(bytes_read) = reader.read() {
        if bytes_read == 0 {
            break;
        }

        let bytes = reader.as_ref();

        for slice in bytes.get(0..bytes_read).unwrap().chunks(columns) {
            line_hexbuf.clear();
            line_buf.clear();

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

            for group in slice.chunks(options.group_size) {
                if options.is_little_endian {
                    for byte in group.iter().rev() {
                        if is_terminal && !options.postscript_style {
                            let colour = get_colour(byte);
                            line_hexbuf.push_str(colour.ansi());
                        }
                        formatter(&mut line_hexbuf, &byte);
                        graphic_bytes += 2;
                    }
                } else {
                    for byte in group.iter() {
                        if is_terminal && !options.postscript_style {
                            let colour = get_colour(byte);
                            line_hexbuf.push_str(colour.ansi());
                        }
                        formatter(&mut line_hexbuf, &byte);
                        graphic_bytes += 2;
                    }
                }

                for byte in group {
                    if is_terminal && !options.postscript_style {
                        let colour = get_colour(byte);
                        line_buf.push_str(colour.ansi());
                    }
                    if !byte.is_ascii_graphic() && *byte != 0x20 {
                        line_buf.push('.')
                    } else {
                        line_buf.push((*byte).into())
                    }
                }

                if !options.postscript_style {
                    line_hexbuf.push(' ');
                    graphic_bytes += 1;
                }
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
                            line_hexbuf,
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

            if !options.postscript_style {
                buffer.push('\n');
            }
            row_counter += 1;

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

    if options.include_format {
        include_format(inhandle, outhandle, options);
        return;
    }

    regular_format(inhandle, outhandle, options, is_terminal);
}
