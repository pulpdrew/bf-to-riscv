A couple of weeks ago, I worked through [Learn Assembly by Writing Entirely Too Many Brainfuck Compilers](https://github.com/pretzelhammer/rust-blog/blob/master/posts/too-many-brainfuck-compilers.md). That article covers the basics of compiling Brainfuck to target x86, ARM, wasm, and LLVM IR. Here, I will cover some of the same information, but with a focus on targeting RISC-V assembly.

## Brainfuck

Brainfuck is an [esoteric programming language](https://en.wikipedia.org/wiki/Esoteric_programming_language), with 8 instructions that manipulate a single global pointer and a 30KB memory space. Brainfuck programs are inscrutable sequences of those instructions that tend to amuse programmers.

|Instruction|description|
|-----------|-----------|
| > | increment the global pointer. |
| < | decrement the global pointer. |
| + | increment the byte of memory that the pointer is currently pointing to. Wrap to 0 on overflow. |
| - | decrement the byte of memory that the pointer is currently pointing to. Wrap to 255 on overflow. |
| . | Write the byte of memory that the pointer is currently pointing to as an ASCII character to stdout |
| , | Read an ASCII character from stdin and store it in the byte that the pointer is currently pointing at |
| [ | Jump to the matching `]` if the pointer is pointing to a byte that is zero |
| ] | Jump to the matching `[` if the pointer is pointing to a byte that is non-zero |

A few other details:
- Any character that is not one of the above instructions is ignored, treated as a comment.
- All the bytes in memory are initialized to 0 when the program starts.
- Moving the pointer outside the 0-29999 range of memory addresses is undefined behavior.

As an example, the following program would move to the second byte of memory, increment it 65 times, and print it out. 65 is 'A' in ASCII, so running the program would result in 'A' being printed to stdout.

```text
>+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++.
```
## Parsing Brainfuck in Rust

We will represent Brainfuck instructions with a Rust Enum, with variants for each instruction. To optimize our programs, instead of storing and executing multiple consecutive, identical instructions, most instructions will have an associated `count` field, indicating how many times in a row that instruction occurred in the source program. So, for example, the Brainfuck program `+++++` would be translated into a single `Instruction::AddByte { count: 5 }`, rather than five `Instruction::AddByte`s in a row. The `[` and `]` instructions do not have `count` fields. Instead, they store the index of the instruction that they will jump to, if they jump. The full enum declaration is:

```rust
#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    AddPtr { count: usize },
    SubPtr { count: usize },
    AddByte { count: usize },
    SubByte { count: usize },
    Read { count: usize },
    Write { count: usize },
    LoopStart { end: usize },
    LoopEnd { start: usize },
}
```

To make parsing a little easier, we will write some methods for this enum. The first constructs an Instruction from a source character. The next three will help us modify the count and target fields while parsing. The last will allow us to compare two `Instructions` based only on their variant, rather than both their variant and their field.

```rust
/// Constructs an Instruction from the given char, if possible
pub fn from_char(c: char) -> Option<Self> {
    match c {
        '>' => Some(Instruction::AddPtr { count: 1 }),
        '<' => Some(Instruction::SubPtr { count: 1 }),
        '+' => Some(Instruction::AddByte { count: 1 }),
        '-' => Some(Instruction::SubByte { count: 1 }),
        ',' => Some(Instruction::Read { count: 1 }),
        '.' => Some(Instruction::Write { count: 1 }),
        '[' => Some(Instruction::LoopStart { end: 0 }),
        ']' => Some(Instruction::LoopEnd { start: 0 }),
        _ => None,
    }
}

/// Increment the count field of this Instruction
pub fn increment(&mut self) {
    match self {
        Instruction::AddPtr { count } => *count += 1,
        Instruction::SubPtr { count } => *count += 1,
        Instruction::AddByte { count } => *count += 1,
        Instruction::SubByte { count } => *count += 1,
        Instruction::Read { count } => *count += 1,
        Instruction::Write { count } => *count += 1,
        _ => panic!("increment_count() called on instruction without count"),
    }
}

/// Indicates whether this Instruction has a count field that can be incremented
pub fn can_increment(&self) -> bool {
    match self {
        Instruction::AddPtr { .. }
        | Instruction::SubPtr { .. }
        | Instruction::AddByte { .. }
        | Instruction::SubByte { .. }
        | Instruction::Read { .. }
        | Instruction::Write { .. } => true,
        Instruction::LoopStart { .. } | Instruction::LoopEnd { .. } => false,
    }
}

/// Sets the target (`start`, `end`) field of a `LoopStart` or `LoopEnd` instruction
pub fn set_target(&mut self, target: usize) {
    match self {
        Instruction::LoopStart { end } => *end = target,
        Instruction::LoopEnd { start } => *start = target,
        _ => panic!("set_target() called on instruction with no target"),
    }
}

pub fn variant_eq(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
}
```

Now that we have an enum to model our instructions, we need a function to transform a Brainfuck source string into a `Vec<Instruction>`, which we can later compile into RISC-V.

```rust
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
            if prev_inst.variant_eq(&next_inst.unwrap()) && prev_inst.can_increment() {
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
```

## RISC-V

RISC-V is an open-source instruction set architecture (ISA). An ISA is a specification for a computer architecture that defines the interface between a processor and the programmer. A processor *implements* the RISC-V ISA when it understands encoded RISC-V assembly code. RISC-V is a relatively new ISA that was developed at UC Berkeley in 2010. Its popularity is growing among hardware and software companies, primarily due to its lack of licensing costs and relative simplicity.

We will be compiling Brainfuck to RISC-V assembly. This means that we need to translate Brainfuck source code into RISC-V assembly code that does the same thing.

Before we do that, let's take a look at what RISC-V Assembly looks like. RISC-V is a register-based ISA. That means that there are some number of registers, and that those registers are the operands for each instruction. Registers are word-sized (32, 64, or 128 bits, depending on which version of RISC-V you are using). RISC-V has 32 registers, serving a variety of purposes.

| Register Names | Register Descriptions |
|----------------|-----------------------|
|`s0, s1, .. s11`| callee-saved registers. Useful for storing values that must persist during function calls.|
|`t0, t1, .. t6` | caller-saved registers. Useful for storing temporary values.|
|`a0, a1, .. a7` | registers for storing function arguments and return values.|
|`zero, ra, sp, tp, pc`| special-purpose registers, which we will ignore.|

We will only use two of these registers to run our Brainfuck programs: `s0` will hold the value of our pointer, and `s1` will hold the value of whatever byte of memory we are manipulating. 


To move bytes between memory and registers, we use load and store instructions:

```python
# Load ('l') the byte ('b') from the address stored in s1 into s0
lb    s0, (s1)

# Treat the byte as unsigned ('u')
lbu   s0, (s1)

# Load the 2 bytes ('h' = half word) starting at address 100 into s0
lh    s0, 100

# Load the 4 bytes ('w' = word) starting at address 100 into t1
lw    t1, 100

# Load the constant ('i' = immediate value) 104 into s1
li    s1, 104

# Load the address of the str: label into s0
la    s0, str

# Store the byte in s1 into the memory address stored in s0
sb    s1, (s0)
```


We will also need some arithmetic to manipulate the memory. Arithmetic operations in RISC-V operate only on registers. To modify a value in memory, first load it into a register, modify the value in the register, and store it back in memory. 

```python
# Add 200 ('i' = immediate value) to s0, store the result in s1
addi  s1, s0, 200

# Add the value in s1 to the value in s0, store the result in s1
add   s1, s1, s0

# Subtract the value in s1 from the value in s0, store in s2
sub   s2, s0, s1

# RISC-V doesn't have a subtract immediate instruction, instead, add a negative.
# Subtract 100 ('i' = immediate value) from the value in s0, store in s1
addi  s1, s0, -100
```

To implement our `LoopStart` and `LoopEnd` behavior, we need a way to conditionally branch, loop, and jump around our programs.
```python
# Branch (go to) the 'end:' label
b end

# Branch to the 'start:' label if s0 == 0. Otherwise continue with the next instruction
beqz s0, start

# Branch to the 'foo:' label if s0 != 0. Otherwise continue with the next instruction
bnez s0, foo

# There are many other conditional branch instructions, we won't need them though.

# This is what a label looks like:
start:
  ...

# RISC-V branch instructions can only go to a label if the address of the label is 
# relatively 'close' the address of the current instruction. To jump farther, we
# will need the unconditional jump instruction.

# Jump to the 'bar:' label
j bar

# We can combine conditional brancing with jumps to conditionally jump. This
# example shows instructions that jumps to 'end:' only if s1 == 0.
beqz s1, start
j end
start:
  ...
```

Finally, to read and write from stdin and stdout, we will need to make system calls. We will use the ReadChar and WriteChar system calls provided by [RARS](https://github.com/TheThirdOne/rars), a RISC-V assembly and emulator program.
```python

# To make a system call, we set register a7 to the system call number. 11 for WriteChar, 12 for ReadChar.
li a7, 11

# For WriteChar, we provide the argument (the char we want to output) in register a0
lbu a0, (s0)  # Supposing the value of s0 is the address of the char we want ot output

# Then we use ecall to make the system call
ecall

# Finally, for ReadChar, we access the returned char in register a0
sb a0, (s0) # Store the char we read at the address stored in s0

# We can use the same process to make the exit(code) system call that allows our program to exit cleanly after execution
li s0, 0    # Exit code 0 is success
li a7, 93   # exit() is system call 93
ecall
```
## Compiling to RISC-V

The first thing we need to do in our RISC-V assembly code is set up the 30KB memory space, and set a0 to point to the beginning of that space. We can do that as follows:
```python
.data     # Indicates the start of a read/write data section of the assembly program
  memory: .space 30000    # 'space 30000' allocates 30K bytes and sets each to 0. 'memory:' is just a label that points to the beginning of those 30K bytes.

.text     # Indicates the start of a read-only code (text) section
  main:   # Indicates where the beginning of the program is
    la s0, memory     # Loads the address of the beginning of the 30KB memory space into register s0, our pointer.
```

Now let's take a look at how we can translate each Brainfuck `Instruction` into RISC-V. Remember that s0 is our pointer, and s1 the register we are using to manipulate our memory.

```python
# AddPtr ( count = 2)
addi s0, s0, 2

# SubPtr ( count = 1)
addi s0, s0, -1

# AddByte ( count = 1)
lbu s1, (s0)
addi s1, s1, 1
sb s1, (s0)

# SubByte ( count = 45)
lbu s1, (s0)
addi s1, s1, -45
sb s1, (s0)

# Read (count = 3)
li a7, 12
ecall
ecall
ecall
sb a0, (s0)

# Write (count = 2)
li a7, 12
lbu a0, (s0)
ecall
ecall

# LoopStart ( start_index = 4, end_index = 108 )
lbu s1, (s0)
bnez s1, start_4
j end_108
start_4:

# Loop End ( start_index = 4, end_index = 108 )
lbu s1, (s0)
beqz s1, end_108
j start_4
end_108:

```

To compile our `Instructions` to RISC-V, we just have to translate each Brainfuck `Instruction` into the sequences from above. Pay attention to the `count` associated with each `Instruction` indicating how many times in a row the instruction appeared in the original Brainfuck program. In Rust, we have:

```rust
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
        output.push_str(&format!("j end_{}\n", end));
        output.push_str(&format!("start_{}:\n\n", index));
      }
      Instruction::LoopEnd { start } => {
        output.push_str("lbu s1, (s0)\n");
        output.push_str(&format!("beqz s1, end_{}\n", index));
        output.push_str(&format!("j start_{}\n", start));
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
```

## Putting it all Together

We now have the tools to parse a Brainfuck source program into a `Vec<Instruction>` and compile a `Vec<Instruction>` to a RISC-V assembly program. Now we just need to glue the two together. Feel free to do that however you would like. Here, I will provide a small `main()` function that implements a makeshift CLI:

```rust
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
```

And that's it, you've completed a Brainfuck to RISC-V compiler. The complete source can be found [here](https://github.com/pulpdrew/bf-to-riscv). If you want to run the output assembly programs, you'll need to download [RARS](https://github.com/TheThirdOne/rars), a RISC-V assembler and emulator. If you want some sample Brainfuck programs to compile (who, after all, would ever want to write one themselves?), several are available from the original [companion code repository](https://github.com/pretzelhammer/brainfuck_compilers/tree/master/input) for "Learn Assembly by Writing Entirely Too Many Brainfuck Compilers."