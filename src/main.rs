use std::{cmp::min, env, fs};
pub mod argparse;

const HELP_TEXT: &str = "
USAGE: rxd [options] [[infile] [outfile]]
";

const VERSION: &str = "0.1";

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

    let mut contents: Vec<u8> = Vec::new();

    let mut inhandle: Box<dyn std::io::Read> = match options.infile.clone() {
        Some(filename) => match fs::File::open(&filename) {
            Err(err) => {
                println!("Could not open {}: {}", &filename, err.to_string());
                return;
            }
            Ok(handle) => Box::new(handle),
        },
        None => Box::new(std::io::stdin()),
    };

    // the size of contents (Vec<u8>) needs to be limited by -len_octets
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

    let mut buffer = String::new();

    let mut line_hexbuf = String::new();
    let mut line_buf = String::new();

    let total_length = contents.len();
    let columns = options.cols.unwrap_or(if options.include_format {
        12
    } else if options.postscript_style {
        30
    } else if options.bits {
        6
    } else {
        16
    });

    let mut output_handle: Box<dyn std::io::Write> = match options.outfile {
        None => Box::new(std::io::stdout()),
        Some(filename) => {
            Box::new(fs::File::create(filename).expect("Could not create output file."))
        }
    };

    // functions to write bytes to the buffer in a particular format follow this line

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

    // Swaps nibbles in a byte. Needed as bytes are usually displayed in big-endian order and we
    // need little endian.
    fn swap_nibbles(byte: u8) -> u8 {
        ((byte & 0x0f) << 4) | ((byte & 0xf0) >> 4)
    }

    // Doing this as branching might be a problem (if dispatch isn't...) and it's easier to manage the code here
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
            if row * columns == total_length {
                continue;
            }

            buffer.push_str("  ");
            for byte in contents[row * columns..min(row * columns + columns, total_length)]
                .as_ref()
                .into_iter()
            {
                let outbyte = swap_nibbles(*byte);
                std::fmt::write(
                    &mut buffer,
                    format_args!("0x{:x}{:x}, ", outbyte & 15, outbyte >> 4 & 15),
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
            .as_ref()
            .iter()
            .enumerate()
        {
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
            let yellow: Vec<u8> = vec![0x1b, 0x5b, 0x39, 0x32, 0x6d].into(); // ESC[92m
            let reset: Vec<u8> = vec![0x1b, 0x5b, 0x30, 0x6d].into(); // ESC[0m
            buffer += format!(
                "{:0>8x}: {}{: <39}  {}{}",
                row * columns,
                String::from_utf8(yellow).unwrap(),
                line_hexbuf,
                line_buf,
                String::from_utf8(reset).unwrap()
            )
            .as_str();
        }

        buffer.push('\n');

        // if size exceeds a page, write it out
        // TODO This does not actually reduce memory usage. Figure out why.
        if buffer.len() >= 4096 {
            // apparently this is line-buffered... so what is consuming memory?
            output_handle
                .write_all(buffer.as_bytes())
                .expect("Could not write to handle.");
            buffer.clear();
        }
    }

    output_handle
        .write_all(buffer.trim().as_bytes())
        .expect("Could not write to handle.");
    println!(""); // final newline
}
