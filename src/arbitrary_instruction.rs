use crate::instruction_handlers::{
    Add, And, CallingMode, Div, Heap, Mul, Or, PtrAdd, PtrPack, PtrShrink, PtrSub, RotateLeft,
    RotateRight, ShiftLeft, ShiftRight, Sub, Xor,
};
use crate::{instruction::Instruction, Predicate};
use arbitrary::Arbitrary;

impl<'a> Arbitrary<'a> for Instruction {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let predicate = if u.arbitrary()? {
            Predicate::Always
        } else {
            u.arbitrary()?
        };

        Ok(match u.choose_index(23)? {
            0 => Self::from_binop::<Add>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            1 => Self::from_binop::<Sub>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            2 => Self::from_binop::<And>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            3 => Self::from_binop::<Or>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            4 => Self::from_binop::<Xor>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            5 => Self::from_binop::<Xor>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            6 => Self::from_binop::<ShiftLeft>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            7 => Self::from_binop::<ShiftRight>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            8 => Self::from_binop::<RotateLeft>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            9 => Self::from_binop::<RotateRight>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                (),
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            10 => Self::from_binop::<Mul>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            11 => Self::from_binop::<Div>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
                u.arbitrary()?,
                u.arbitrary()?,
            ),
            12 => Self::from_jump(u.arbitrary()?, predicate),
            13 => Self::from_ptr::<PtrAdd>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
                u.arbitrary()?,
            ),
            14 => Self::from_ptr::<PtrSub>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
                u.arbitrary()?,
            ),
            15 => Self::from_ptr::<PtrShrink>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
                u.arbitrary()?,
            ),
            16 => Self::from_ptr::<PtrPack>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
                u.arbitrary()?,
            ),
            17 => {
                Self::from_load::<Heap>(u.arbitrary()?, u.arbitrary()?, u.arbitrary()?, predicate)
            }
            18 => Self::from_store::<Heap>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
                false,
            ),
            19 => {
                Self::from_load_pointer(u.arbitrary()?, u.arbitrary()?, u.arbitrary()?, predicate)
            }
            20 => Self::from_sstore(u.arbitrary()?, u.arbitrary()?, predicate),
            21 => Self::from_sload(u.arbitrary()?, u.arbitrary()?, predicate),
            22 => Self::from_far_call::<{ CallingMode::Normal as u8 }>(
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                u.arbitrary()?,
                predicate,
            ),
            _ => unreachable!(),
        })
    }
}
