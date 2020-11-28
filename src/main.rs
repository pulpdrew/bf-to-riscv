use std::{env, fs, fs::File, io::Read};

use brainfuck_riscv::{compiler::compile_risc_v, parser::parse};

/// The message shown to the user when they type a command incorrectly
const USAGE_MESSAGE: &str = "Usage: bf <input> [-o <output>]";

/// The default filename of the output file
const DEFAULT_OUTPUT_FILENAME: &str = "out.asm";

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_filename = args.get(1).expect(USAGE_MESSAGE);

    // Get the output filename, if provided. Otherwise, use the default.
    let output_index = args.iter().position(|s| s == "-o" || s == "--output");
    let output_filename = if let Some(index) = output_index {
        args.get(index + 1).expect(USAGE_MESSAGE)
    } else {
        DEFAULT_OUTPUT_FILENAME
    };

    // Read the source file
    let mut source = String::new();
    let mut input_file = File::open(input_filename).expect("Failed to open source file");
    input_file
        .read_to_string(&mut source)
        .expect("Failed to read source file");

    // Compile and write to output
    let program = parse(&source);
    let output = compile_risc_v(&program);
    fs::write(output_filename, output).expect("Failed to write output");
}
