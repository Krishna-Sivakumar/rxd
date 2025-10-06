use std::{cmp::min, env, fs};
pub mod argparse;

const HELP_TEXT: &str = "
USAGE: rxd [options] [[infile] [outfile]]
";

const VERSION: &str = "0.1";

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

    let filename = match options.infile.clone() {
        Some(filename) => filename,
        None => {
            println!("Expected an input filename.");
            return;
        }
    };

    let contents = match fs::read_to_string(&filename) {
        Ok(val) => val[0..options.len_octets.unwrap_or(val.len())].to_owned(),
        Err(_) => {
            println!("file \"{}\" not found.", filename);
            return;
        }
    };

    let mut buffer = String::new();

    let mut line_hexbuf = String::new();
    let mut line_buf = String::new();

    let total_length = options.len_octets.unwrap_or(contents.len());
    let columns = if options.postscript_style {
        30
    } else if options.include_format {
        12
    } else {
        options.cols
    };

    let mut output_handle: Box<dyn std::io::Write> = match options.outfile {
        None => Box::new(std::io::stdout()),
        Some(filename) => {
            Box::new(fs::File::create(filename).expect("Could not create output file."))
        }
    };

    fn to_lower_hex(buffer: &mut String, byte: &u8) {
        std::fmt::write(buffer, format_args!("{:x}{:x}", byte & 15, byte >> 4 & 15))
            .expect("write must succeed.");
    }

    fn to_upper_hex(buffer: &mut String, byte: &u8) {
        std::fmt::write(buffer, format_args!("{:X}{:X}", byte & 15, byte >> 4 & 15))
            .expect("write must succeed.");
    }

    fn to_binary(buffer: &mut String, byte: &u8) {
        std::fmt::write(buffer, format_args!("{:b}{:b}", byte & 15, byte >> 4 & 15))
            .expect("write must succeed.");
    }

    let formatter = if options.bits {
        to_binary
    } else if options.uppercase {
        to_upper_hex
    } else {
        to_lower_hex
    };

    if options.include_format {
        let include_filename = options.include_name.unwrap_or("buffer".into());
        buffer.push_str(format!("unsigned char {}[] = {{\n", include_filename).as_str());

        for row in 0..(total_length / columns + 1) {
            // TODO render out in little-endian
            if row * columns == total_length {
                continue;
            }

            buffer.push_str("  ");
            for byte in contents[row * columns..min(row * columns + columns, total_length)].bytes()
            {
                std::fmt::write(
                    &mut buffer,
                    format_args!("0x{:x}{:x}, ", byte & 15, byte >> 4 & 15),
                )
                .expect("write must succeed.");
            }
            buffer.push('\n');
        }

        buffer.push_str(
            format!(
                "}};\nunsigned int {} = {};",
                include_filename + "_len",
                contents.len()
            )
            .as_str(),
        );

        output_handle
            .write_all(buffer.as_bytes())
            .expect("Could not write to handle.");

        return;
    }

    for row in 0..(total_length / columns + 1) {
        line_hexbuf.clear();
        line_buf.clear();

        for (idx, byte) in contents[row * columns..min(row * columns + columns, total_length)]
            .bytes()
            .enumerate()
        {
            if byte.is_ascii_whitespace() && byte != 32 {
                line_buf.push('.')
            } else {
                line_buf.push(byte.into())
            }

            if !options.postscript_style && idx % options.group_size == 0 && idx > 0 {
                line_hexbuf.push(' ')
            }

            formatter(&mut line_hexbuf, &byte);
        }

        if options.postscript_style {
            // TODO display format needs to be little endian in this case
            buffer += line_hexbuf.as_str();
        } else {
            buffer +=
                format!("{:0>8x}: {: <39}  {}", row * columns, line_hexbuf, line_buf).as_str();
        }

        buffer.push('\n');
    }

    output_handle
        .write_all(buffer.as_bytes())
        .expect("Could not write to handle.");
}
