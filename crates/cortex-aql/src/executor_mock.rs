use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::binder::{BitmapHandle, BitmapOp, BitmapProgram};

pub use roaring::RoaringBitmap;

pub trait BitmapProvider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<RoaringBitmap>;
    fn agent_allowed(&self) -> RoaringBitmap;
    fn live(&self) -> RoaringBitmap;
    fn universe(&self) -> RoaringBitmap;
}

#[derive(Clone, Debug, Default)]
pub struct MockBitmapProvider {
    pub bitmaps: BTreeMap<BitmapHandle, BTreeSet<u32>>,
    pub agent_allowed: BTreeSet<u32>,
    pub live: BTreeSet<u32>,
    pub universe: BTreeSet<u32>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BitmapVmError {
    #[error("bitmap handle not found: {0:?}")]
    MissingBitmap(BitmapHandle),
    #[error("bitmap stack underflow")]
    StackUnderflow,
    #[error("bitmap program ended with invalid stack")]
    InvalidFinalStack,
}

pub fn eval_bitmap_program<P: BitmapProvider>(
    program: &BitmapProgram,
    provider: &P,
) -> Result<BTreeSet<u32>, BitmapVmError> {
    Ok(eval_bitmap_program_bitmap(program, provider)?
        .iter()
        .collect())
}

pub fn eval_bitmap_program_bitmap<P: BitmapProvider>(
    program: &BitmapProgram,
    provider: &P,
) -> Result<RoaringBitmap, BitmapVmError> {
    let mut stack = Vec::<RoaringBitmap>::with_capacity(program.max_stack_depth);
    for op in &program.ops {
        match *op {
            BitmapOp::Push(handle) => stack.push(
                provider
                    .bitmap(handle)
                    .ok_or(BitmapVmError::MissingBitmap(handle))?,
            ),
            BitmapOp::PushUniverse => stack.push(provider.universe()),
            BitmapOp::PushAgentAllowed => stack.push(provider.agent_allowed()),
            BitmapOp::PushLive => stack.push(provider.live()),
            BitmapOp::And => {
                let rhs = pop(&mut stack)?;
                let mut lhs = pop(&mut stack)?;
                lhs &= &rhs;
                stack.push(lhs);
            }
            BitmapOp::Or => {
                let rhs = pop(&mut stack)?;
                let mut lhs = pop(&mut stack)?;
                lhs |= &rhs;
                stack.push(lhs);
            }
            BitmapOp::Not => {
                let current = pop(&mut stack)?;
                let mut universe = provider.universe();
                universe -= &current;
                stack.push(universe);
            }
        }
    }
    if stack.len() == 1 {
        Ok(stack.pop().expect("checked stack length"))
    } else {
        Err(BitmapVmError::InvalidFinalStack)
    }
}

fn pop(stack: &mut Vec<RoaringBitmap>) -> Result<RoaringBitmap, BitmapVmError> {
    stack.pop().ok_or(BitmapVmError::StackUnderflow)
}

impl BitmapProvider for MockBitmapProvider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<RoaringBitmap> {
        self.bitmaps.get(&handle).map(bitmap_from_set)
    }

    fn agent_allowed(&self) -> RoaringBitmap {
        bitmap_from_set(&self.agent_allowed)
    }

    fn live(&self) -> RoaringBitmap {
        bitmap_from_set(&self.live)
    }

    fn universe(&self) -> RoaringBitmap {
        bitmap_from_set(&self.universe)
    }
}

fn bitmap_from_set(values: &BTreeSet<u32>) -> RoaringBitmap {
    values.iter().copied().collect()
}
