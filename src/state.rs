use crate::{
    addressing_modes::{Addressable, Arguments},
    bitset::Bitset,
    decommit::{address_into_u256, decommit, u256_into_address},
    fat_pointer::FatPointer,
    instruction_handlers::CallingMode,
    modified_world::ModifiedWorld,
    predication::Flags,
    Predicate, World,
};
use arbitrary::{Arbitrary, Unstructured};
use std::sync::Arc;
use u256::{H160, U256};

pub struct State {
    pub world: ModifiedWorld,

    pub registers: [U256; 16],
    pub(crate) register_pointer_flags: u16,

    pub flags: Flags,

    pub current_frame: Callframe,

    /// Contains pointers to the far call instructions currently being executed.
    /// They are needed to continue execution from the correct spot upon return.
    previous_frames: Vec<(*const Instruction, Callframe)>,

    pub(crate) heaps: Vec<Vec<u8>>,

    context_u128: u128,
}

pub struct Callframe {
    pub address: H160,
    pub code_address: H160,
    pub caller: H160,
    pub program: Arc<[Instruction]>,
    pub code_page: Arc<[U256]>,
    context_u128: u128,

    // TODO: joint allocate these.
    pub stack: Box<[U256; 1 << 16]>,
    pub stack_pointer_flags: Box<Bitset>,

    pub heap: u32,
    pub aux_heap: u32,

    pub sp: u16,
    pub gas: u32,

    near_calls: Vec<(*const Instruction, u16, u32)>,
}

impl Addressable for State {
    fn registers(&mut self) -> &mut [U256; 16] {
        &mut self.registers
    }
    fn register_pointer_flags(&mut self) -> &mut u16 {
        &mut self.register_pointer_flags
    }
    fn stack(&mut self) -> &mut [U256; 1 << 16] {
        &mut self.current_frame.stack
    }
    fn stack_pointer_flags(&mut self) -> &mut Bitset {
        &mut self.current_frame.stack_pointer_flags
    }
    fn stack_pointer(&mut self) -> &mut u16 {
        &mut self.current_frame.sp
    }
    fn code_page(&self) -> &[U256] {
        &self.current_frame.code_page
    }
}

impl Callframe {
    fn new(
        address: H160,
        code_address: H160,
        caller: H160,
        program: Arc<[Instruction]>,
        code_page: Arc<[U256]>,
        heap: u32,
        aux_heap: u32,
        gas: u32,
        context_u128: u128,
    ) -> Self {
        Self {
            address,
            code_address,
            caller,
            program,
            context_u128,
            stack: vec![U256::zero(); 1 << 16]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            stack_pointer_flags: Default::default(),
            code_page,
            heap,
            aux_heap,
            sp: 1024,
            gas,
            near_calls: vec![],
        }
    }

    pub(crate) fn push_near_call(&mut self, gas_to_call: u32, old_pc: *const Instruction) {
        self.near_calls
            .push((old_pc, self.sp, self.gas - gas_to_call));
        self.gas = gas_to_call;
    }

    pub(crate) fn pop_near_call(&mut self) -> Option<*const Instruction> {
        self.near_calls.pop().map(|(pc, sp, gas)| {
            self.sp = sp;
            self.gas = gas;
            pc
        })
    }
}

pub struct Instruction {
    pub(crate) handler: Handler,
    pub(crate) arguments: Arguments,
}

pub(crate) type Handler = fn(&mut State, *const Instruction) -> ExecutionResult;
pub type ExecutionResult = Result<Vec<u8>, Panic>;

#[derive(Debug)]
pub enum Panic {
    OutOfGas,
    IncorrectPointerTags,
    PointerOffsetTooLarge,
    PtrPackLowBitsNotZero,
    JumpingOutOfProgram,
}

impl State {
    pub fn new(mut world: Box<dyn World>, address: H160, caller: H160, calldata: Vec<u8>) -> Self {
        let (program, code_page) = decommit(&mut *world, address_into_u256(address));
        let mut registers: [U256; 16] = Default::default();
        registers[1] = FatPointer {
            memory_page: 1,
            offset: 0,
            start: 0,
            length: calldata.len() as u32,
        }
        .into_u256();
        Self {
            world: ModifiedWorld::new(world),
            registers,
            register_pointer_flags: 1 << 1, // calldata is a pointer
            flags: Flags::new(false, false, false),
            current_frame: Callframe::new(
                address,
                address,
                caller,
                program,
                code_page,
                2,
                3,
                u32::MAX,
                0,
            ),
            previous_frames: vec![],

            // The first heap can never be used because heap zero
            // means the current heap in precompile calls
            heaps: vec![vec![], calldata, vec![], vec![]],
            context_u128: 0,
        }
    }

    pub(crate) fn push_frame<const CALLING_MODE: u8>(
        &mut self,
        instruction_pointer: *const Instruction,
        code_address: H160,
        program: Arc<[Instruction]>,
        code_page: Arc<[U256]>,
        gas: u32,
    ) {
        let new_heap = self.heaps.len() as u32;
        self.heaps.extend([vec![], vec![]]);
        let mut new_frame = Callframe::new(
            if CALLING_MODE == CallingMode::Delegate as u8 {
                self.current_frame.address
            } else {
                code_address
            },
            code_address,
            if CALLING_MODE == CallingMode::Normal as u8 {
                self.current_frame.address
            } else if CALLING_MODE == CallingMode::Delegate as u8 {
                self.current_frame.caller
            } else {
                u256_into_address(self.registers[3])
            },
            program,
            code_page,
            new_heap,
            new_heap + 1,
            gas,
            if CALLING_MODE == CallingMode::Delegate as u8 {
                self.current_frame.context_u128
            } else {
                self.context_u128
            },
        );
        self.context_u128 = 0;

        std::mem::swap(&mut new_frame, &mut self.current_frame);
        self.previous_frames.push((instruction_pointer, new_frame));
    }

    pub(crate) fn pop_frame(&mut self) -> Option<*const Instruction> {
        self.previous_frames.pop().map(|(pc, frame)| {
            self.current_frame = frame;
            pc
        })
    }

    pub(crate) fn set_context_u128(&mut self, value: u128) {
        self.context_u128 = value;
    }

    pub(crate) fn get_context_u128(&self) -> u128 {
        self.current_frame.context_u128
    }

    pub fn run(&mut self) -> ExecutionResult {
        let instruction: *const Instruction = &self.current_frame.program[0];
        self.run_starting_from(instruction)
    }

    pub(crate) fn run_starting_from(
        &mut self,
        mut instruction: *const Instruction,
    ) -> ExecutionResult {
        self.use_gas(1)?;

        // Instructions check predication for the *next* instruction, not the current one.
        // Thus, we can't just blindly run the first instruction.
        unsafe {
            while !(*instruction).arguments.predicate.satisfied(&self.flags) {
                instruction = instruction.add(1);
                self.use_gas(1)?;
            }
            ((*instruction).handler)(self, instruction)
        }
    }

    #[inline(always)]
    pub(crate) fn use_gas(&mut self, amount: u32) -> Result<(), Panic> {
        if self.current_frame.gas >= amount {
            self.current_frame.gas -= amount;
            Ok(())
        } else {
            Err(Panic::OutOfGas)
        }
    }
}

pub fn end_execution() -> Instruction {
    Instruction {
        handler: end_execution_handler,
        arguments: Arguments::new(Predicate::Always),
    }
}
fn end_execution_handler(_state: &mut State, _: *const Instruction) -> ExecutionResult {
    Ok(vec![])
}

pub fn jump_to_beginning() -> Instruction {
    Instruction {
        handler: jump_to_beginning_handler,
        arguments: Arguments::new(Predicate::Always),
    }
}
fn jump_to_beginning_handler(state: &mut State, _: *const Instruction) -> ExecutionResult {
    let first_instruction = &state.current_frame.program[0];
    let first_handler = first_instruction.handler;
    first_handler(state, first_instruction)
}

pub fn run_arbitrary_program(input: &[u8]) -> ExecutionResult {
    let mut u = Unstructured::new(input);
    let mut program: Vec<Instruction> = Arbitrary::arbitrary(&mut u).unwrap();

    if program.len() >= 1 << 16 {
        program.truncate(1 << 16);
        program.push(jump_to_beginning());
    } else {
        // TODO execute invalid instruction or something instead
        program.push(end_execution());
    }

    struct FakeWorld;
    impl World for FakeWorld {
        fn decommit(&mut self, hash: U256) -> (Arc<[Instruction]>, Arc<[U256]>) {
            todo!()
        }

        fn read_storage(&mut self, _: H160, _: U256) -> U256 {
            U256::zero()
        }
    }

    let mut state = State::new(Box::new(FakeWorld), H160::zero(), H160::zero(), vec![]);
    state.run()
}
