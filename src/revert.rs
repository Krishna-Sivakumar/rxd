use crate::argparse::Options;

pub fn revert(
    inhandle: Box<dyn std::io::Read>,
    outhandle: Box<dyn std::io::Write>,
    options: Options,
) {
    /* parsing outline:
     * the first token is considered as an offset. This needs to be encoded in hex.
     * If we're writing to a file, seek to the offset.
     *
     * Read the `options.cols * options.group_size` bytes separated by a singular space.
     * At this point, move on to the next line.
     *
     * If the parse of a line offset fails, move on to the next line offset token.
     * If the parse of a byte fails, move on to the next byte token.
     */
}
