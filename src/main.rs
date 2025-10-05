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
        Ok(val) => val,
        Err(_) => {
            println!("file \"{}\" not found.", filename);
            return;
        }
    };

    let mut buffer = String::new();

    let mut line_hexbuf = String::new();
    let mut line_buf = String::new();

    let total_length = options.len_octets.unwrap_or(contents.len());

    for row in 0..(total_length / options.cols + 1) {
        line_hexbuf.clear();
        line_buf.clear();

        for (idx, byte) in contents
            [row * options.cols..min(row * options.cols + options.cols, total_length)]
            .bytes()
            .enumerate()
        {
            if byte.is_ascii_whitespace() && byte != 32 {
                line_buf.push('.')
            } else {
                line_buf.push(byte.into())
            }

            if idx % options.group_size == 0 && idx > 0 {
                line_hexbuf.push(' ')
            }

            let mut outbyte = byte;
            if options.is_little_endian {
                outbyte = outbyte.to_le();
            } else {
                outbyte = outbyte.to_be();
            }

            if options.bits {
                line_hexbuf.push_str(format!("{:b}", outbyte & 15).as_str());
                line_hexbuf.push_str(format!("{:b}", outbyte >> 4 & 15).as_str());
            } else {
                if options.uppercase {
                    line_hexbuf.push_str(format!("{:X}", outbyte & 15).as_str());
                    line_hexbuf.push_str(format!("{:X}", outbyte >> 4 & 15).as_str());
                } else {
                    line_hexbuf.push_str(format!("{:x}", outbyte & 15).as_str());
                    line_hexbuf.push_str(format!("{:x}", outbyte >> 4 & 15).as_str());
                }
            }
        }

        buffer += format!(
            "{:0>8x}: {: <39}  {}",
            row * options.cols,
            line_hexbuf,
            line_buf
        )
        .as_str();
        buffer.push('\n');
    }

    print!("{}", buffer);
}
