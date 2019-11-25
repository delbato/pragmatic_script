pub use stack_vm::{
    InstructionTable,
    Machine,
    Builder,
    Instruction,
    WriteManyTable,
    Code
};

type IntOperand = i64;


fn push_int(machine: &mut Machine<IntOperand>, args: &[usize]) {
    let arg = machine.get_data(args[0]).clone();
    machine.operand_push(arg);
}

fn add_int(machine: &mut Machine<IntOperand>, args: &[usize]) {
    let lhs = machine.operand_pop().clone();
    let rhs = machine.operand_pop().clone();
    machine.operand_push(lhs + rhs);
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_int_add() {
        let mut instruction_table = InstructionTable::new();
        instruction_table.insert(Instruction::new(0, "pushi", 1, push_int));
        instruction_table.insert(Instruction::new(1, "addi", 0, add_int));

        let mut builder: Builder<IntOperand> = Builder::new(&instruction_table);
        builder.push("pushi", vec![4]);
        builder.push("pushi", vec![6]);
        builder.push("addi", vec![]);

        let constants: WriteManyTable<IntOperand> = WriteManyTable::new();
        let mut machine = Machine::new(Code::from(builder), &constants, &instruction_table);
        machine.run();
        
        assert_eq!(machine.operand_pop(), 10);
    }
}
