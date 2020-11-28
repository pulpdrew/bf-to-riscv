use crate::instruction::Instruction;

/// Parse a Brainfuck source string
pub fn parse(source: &str) -> Vec<Instruction> {
    // The sequence of parsed instruction, to be returned
    let mut program: Vec<Instruction> = Vec::new();

    // Indices in `program` at which loops start
    let mut loop_starts: Vec<usize> = Vec::new();

    for command in source.chars().filter(|c| "<>+-,.[]".contains(*c)) {
        // Convert the character to an instruction
        let mut next_inst = Instruction::from_char(command);

        // If the new instruction is the same as the previous, just increment the count in the previous
        if let Some(prev_inst) = program.last_mut() {
            if prev_inst == &next_inst.unwrap() && prev_inst.can_increment() {
                prev_inst.increment();
                next_inst = None;
            }
        };

        // Set loop start / loop end targets
        if let Some(Instruction::LoopStart { .. }) = next_inst {
            loop_starts.push(program.len());
        } else if let Some(Instruction::LoopEnd { start }) = &mut next_inst {
            let loop_start = loop_starts.pop().expect("Unmatched loop end");
            *start = loop_start;

            let loop_end = program.len();
            program.get_mut(loop_start).unwrap().set_target(loop_end);
        }

        // Add the next instruction to the program (as long as it wasn't the same as the previous)
        if let Some(i) = next_inst {
            program.push(i);
        }
    }

    // Error if there are more loop starts than ends
    if !loop_starts.is_empty() {
        panic!("Unmatched loop start.");
    }

    program
}
