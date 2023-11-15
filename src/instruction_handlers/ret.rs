use super::call::get_far_call_arguments;
use crate::{
    addressing_modes::{Arguments, Register1},
    predication::Flags,
    state::{ExecutionEnd, InstructionResult, Panic},
    Instruction, Predicate, State,
};
use u256::U256;

fn ret<const IS_REVERT: bool>(
    state: &mut State,
    instruction: *const Instruction,
) -> InstructionResult {
    let args = unsafe { &(*instruction).arguments };

    let gas_left = state.current_frame.gas;

    let (pc, eh) = if let Some(pc_eh) = state.current_frame.pop_near_call() {
        pc_eh
    } else {
        let return_value = match get_far_call_arguments(args, state) {
            Ok(abi) => abi.pointer,
            Err(panic) => return ret_panic(state, panic),
        };

        // TODO check that the return value resides in this or a newer frame's memory

        let Some(pc_eh) = state.pop_frame() else {
            let output = state.heaps[return_value.memory_page as usize][return_value.start as usize..(return_value.start + return_value.length) as usize].to_vec();
            return if IS_REVERT{
                Err(ExecutionEnd::Reverted(output))
            } else {
                Err(ExecutionEnd::ProgramFinished(output))
            };
        };

        state.set_context_u128(0);

        state.registers = [U256::zero(); 16];
        state.registers[1] = return_value.into_u256();
        state.register_pointer_flags = 2;
        pc_eh
    };

    state.flags = Flags::new(false, false, false);
    state.current_frame.gas += gas_left;

    if IS_REVERT {
        match state.current_frame.program.get(eh as usize) {
            Some(i) => Ok(i),
            None => ret_panic(state, Panic::InvalidInstruction),
        }
    } else {
        Ok(unsafe { pc.add(1) })
    }
}

pub(crate) fn ret_panic(state: &mut State, mut panic: Panic) -> InstructionResult {
    loop {
        let eh = if let Some((_, eh)) = state.current_frame.pop_near_call() {
            eh
        } else {
            let Some((_, eh)) = state.pop_frame() else {
                return Err(ExecutionEnd::Panicked(panic));
            };
            state.registers[1] = U256::zero();
            state.register_pointer_flags |= 1 << 1;
            eh
        };

        state.flags = Flags::new(true, false, false);
        state.set_context_u128(0);

        if let Some(instruction) = state.current_frame.program.get(eh as usize) {
            return Ok(instruction);
        }
        panic = Panic::InvalidInstruction;
    }
}

impl Instruction {
    pub fn from_ret(src1: Register1, predicate: Predicate) -> Self {
        Self {
            handler: ret::<false>,
            arguments: Arguments::new(predicate, 5).write_source(&src1),
        }
    }
    pub fn from_revert(src1: Register1, predicate: Predicate) -> Self {
        Self {
            handler: ret::<true>,
            arguments: Arguments::new(predicate, 5).write_source(&src1),
        }
    }
}
