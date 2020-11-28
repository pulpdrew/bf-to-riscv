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

impl Instruction {
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

    /// Sets the target (`start`, `end`) field of a `LoopStart` or `LoopEnd` instruction
    pub fn set_target(&mut self, target: usize) {
        match self {
            Instruction::LoopStart { end } => *end = target,
            Instruction::LoopEnd { start } => *start = target,
            _ => panic!("set_target() called on instruction with no target"),
        }
    }
}

/// Implementation of PartialEq for Instruction that compares only the variant, not the field value
impl PartialEq for Instruction {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}
