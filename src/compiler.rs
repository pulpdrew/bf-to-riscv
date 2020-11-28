use crate::instruction::Instruction;

/// Compile the given program into risc v assembly
pub fn compile_risc_v(program: &[Instruction]) -> String {
    let mut output = String::new();

    // Generate code to allocate the 30KB memory space
    output.push_str(".data\n");
    output.push_str("memory: .space 30000\n\n");

    // Register s0 will be our pointer. Set it to point to the beginning of memory
    output.push_str(".text\n");
    output.push_str("main:\n");
    output.push_str("la s0, memory\n");

    // Generate assembly for each instruction
    for (index, inst) in program.iter().enumerate() {
        match inst {
            Instruction::AddPtr { count } => {
                output.push_str(&format!("addi s0, s0, {}\n\n", count));
            }
            Instruction::SubPtr { count } => {
                output.push_str(&format!("addi s0, s0, {}\n\n", -(*count as i64)));
            }
            Instruction::AddByte { count } => {
                output.push_str("lbu s1, (s0)\n");
                output.push_str(&format!("addi s1, s1, {}\n", count));
                output.push_str("sb s1, (s0)\n\n");
            }
            Instruction::SubByte { count } => {
                output.push_str("lbu s1, (s0)\n");
                output.push_str(&format!("addi s1, s1, {}\n", -(*count as i64)));
                output.push_str("sb s1, (s0)\n\n");
            }
            Instruction::Read { count } => {
                output.push_str("li a7, 12\n");

                for _ in 0..*count {
                    output.push_str("ecall\n");
                }

                output.push_str("sb a0, (s0)\n\n")
            }
            Instruction::Write { count } => {
                output.push_str("lbu a0, (s0)\n");
                output.push_str("li a7, 11\n");

                for _ in 0..*count {
                    output.push_str("ecall\n");
                }

                output.push('\n');
            }
            Instruction::LoopStart { end } => {
                output.push_str("lbu s1, (s0)\n");
                output.push_str(&format!("bnez s1, start_{}\n", index));
                output.push_str(&format!("la t0, end_{}\n", end));
                output.push_str("jr t0\n");
                output.push_str(&format!("start_{}:\n\n", index));
            }
            Instruction::LoopEnd { start } => {
                output.push_str("lbu s1, (s0)\n");
                output.push_str(&format!("beqz s1, end_{}\n", index));
                output.push_str(&format!("la t0, start_{}\n", start));
                output.push_str("jr t0\n");
                output.push_str(&format!("end_{}:\n\n", index));
            }
        }
    }

    // Generate code to exit
    output.push_str("li	a0, 0\n");
	output.push_str("li 	a7, 93\n");
	output.push_str("ecall\n\n");

    output
}
