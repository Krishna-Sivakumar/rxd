#[derive(Debug)]
pub struct Options {
    /// TODO toggles autoskip. A single '*' replaces NUL-lines.
    pub autoskip: bool,
    /// Switches to binary dump instead of hex dump.
    pub bits: bool,
    /// Specifies the number of columns in the hex dump.
    pub cols: Option<usize>,
    /// Capitalize variable names in C include file style, when using -i
    pub capitalize: bool,
    /// Switch to little-endian hex dump.
    pub is_little_endian: bool,
    /// the size of a group of bytes in the hexdump. Default is 2.
    pub group_size: usize,
    /// Display help and exit.
    pub display_help: bool,
    /// Output result in C include file style.
    pub include_format: bool,
    /// Stop after writing <len_octets> octets.
    pub len_octets: Option<usize>,
    /// Override the variable name output when -i is used. The array is named <include_name>.
    pub include_name: Option<String>,
    /// Add <offset> to the displayed file position.
    pub offset: usize,
    /// Output in PostScript continuous hex dump style. Also known as plain hex dump style.
    pub postscript_style: bool,
    /// TODO Convert hex dump to binary.
    pub revert: bool,
    /// Start at <seek> bytes.
    pub seek: i32,
    /// Use upper-case hex letters.
    pub uppercase: bool,
    /// Display version number and exit.
    pub display_version: bool,
    //// Input file to read from.
    pub infile: Option<String>,
    /// Output file to write to.
    pub outfile: Option<String>,
}

impl Options {
    pub fn default() -> Self {
        Options {
            autoskip: false,
            bits: false,
            cols: None,
            capitalize: false,
            is_little_endian: false,
            group_size: 2,
            display_help: false,
            include_format: false,
            len_octets: None,
            include_name: None,
            offset: 0,
            postscript_style: false,
            revert: false,
            seek: 0,
            uppercase: false,
            display_version: false,
            infile: None,
            outfile: None,
        }
    }

    /// Parses a list of arguments from the command line and returns Options.
    /// Grammmar:
    /// [binary-name] [-r[evert]] [options] [[infile] [outfile]]
    pub fn parse_options(arguments: Vec<String>) -> Result<Self, String> {
        let mut options = Options::default();
        let mut arg: usize = 0;

        /// Get the next argument in the list and check if it is a number.
        /// If the next argument doesn't exist or if we can't parse the argument, error out.
        fn take<T: std::str::FromStr>(arguments: &Vec<String>, arg: &usize) -> Option<T> {
            arguments
                .get(arg + 1)
                .and_then(|next_arg| next_arg.parse::<T>().ok())
        }

        while arg < arguments.len() {
            let argument = arguments.get(arg).expect("this cannot fail");
            if argument.starts_with("-") {
                match argument.as_str() {
                    "-a" | "-autoskip" => options.autoskip = true,
                    "-b" | "-bits" => options.bits = true,
                    "-c" | "-cols" => match take(&arguments, &arg) {
                        None => {
                            return Err("-cols requires an integer value following it.".to_owned());
                        }
                        Some(cols) => {
                            options.cols = Some(std::cmp::max(1, cols));
                            arg += 1;
                        }
                    },
                    "-C" | "-capitalize" => options.capitalize = true,
                    "-e" => options.is_little_endian = true,
                    "-g" | "-groupsize" => match take::<usize>(&arguments, &arg) {
                        None => {
                            return Err(
                                "-groupsize requires an integer value following it.".to_owned()
                            );
                        }
                        Some(g) => {
                            options.group_size = g.clamp(1, 16); //std::cmp::max(1, std::cmp::min(16, g));
                            arg += 1;
                        }
                    },
                    "-h" | "-help" => options.display_help = true,
                    "-i" | "-include" => options.include_format = true,
                    "-l" | "-len" => match take(&arguments, &arg) {
                        None => {
                            return Err("-len requires an integer value following it.".to_owned());
                        }
                        Some(len_octets) => {
                            options.len_octets = Some(len_octets);
                            arg += 1;
                        }
                    },
                    "-n" | "-name" => match take(&arguments, &arg) {
                        None => {
                            return Err("-name requires an array name following it.".to_owned());
                        }
                        Some(name) => {
                            options.include_name = Some(name);
                            arg += 1;
                        }
                    },
                    "-o" => match take(&arguments, &arg) {
                        None => return Err("-o requires an integer value following it.".to_owned()),
                        Some(offset) => {
                            options.offset = offset;
                            arg += 1;
                        }
                    },
                    "-p" | "-ps" | "-postscript" | "-plain" => options.postscript_style = true,
                    "-r" | "-revert" => options.revert = true,
                    "-seek" => match take(&arguments, &arg) {
                        None => {
                            return Err(
                                "-seek requires an unsigned offset following it.".to_owned()
                            );
                        }
                        Some(offset) => options.seek = offset,
                    },
                    "-s" => match take(&arguments, &arg) {
                        None => {
                            return Err("-s requires an integer offset following it.".to_owned());
                        }
                        Some(offset) => {
                            options.seek = offset;
                            arg += 1;
                        }
                    },
                    "-u" => options.uppercase = true,
                    "-v" => options.display_version = true,
                    option => return Err(format!("{} is not an option.", option)),
                }
            } else {
                break;
            }
            arg += 1;
        }

        if arg < arguments.len() {
            options.infile = Some(arguments.get(arg).expect("this cannot fail").clone());
            arg += 1;
        }

        if arg < arguments.len() {
            options.outfile = Some(arguments.get(arg).expect("this cannot fail").clone());
        }

        Ok(options)
    }
}
