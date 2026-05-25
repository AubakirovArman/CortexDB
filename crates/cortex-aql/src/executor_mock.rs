use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::binder::{BitmapHandle, BitmapOp, BitmapProgram};

pub trait BitmapProvider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<BTreeSet<u32>>;
    fn agent_allowed(&self) -> BTreeSet<u32>;
    fn live(&self) -> BTreeSet<u32>;
    fn universe(&self) -> BTreeSet<u32>;
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
    let mut stack = Vec::<BTreeSet<u32>>::with_capacity(program.max_stack_depth);
    for op in &program.ops {
        match *op {
            BitmapOp::Push(handle) => stack.push(
                provider
                    .bitmap(handle)
                    .ok_or(BitmapVmError::MissingBitmap(handle))?,
            ),
            BitmapOp::PushAgentAllowed => stack.push(provider.agent_allowed()),
            BitmapOp::PushLive => stack.push(provider.live()),
            BitmapOp::And => {
                let rhs = pop(&mut stack)?;
                let lhs = pop(&mut stack)?;
                stack.push(lhs.intersection(&rhs).copied().collect());
            }
            BitmapOp::Or => {
                let rhs = pop(&mut stack)?;
                let lhs = pop(&mut stack)?;
                stack.push(lhs.union(&rhs).copied().collect());
            }
            BitmapOp::Not => {
                let current = pop(&mut stack)?;
                stack.push(provider.universe().difference(&current).copied().collect());
            }
        }
    }
    if stack.len() == 1 {
        Ok(stack.pop().expect("checked stack length"))
    } else {
        Err(BitmapVmError::InvalidFinalStack)
    }
}

fn pop(stack: &mut Vec<BTreeSet<u32>>) -> Result<BTreeSet<u32>, BitmapVmError> {
    stack.pop().ok_or(BitmapVmError::StackUnderflow)
}

impl BitmapProvider for MockBitmapProvider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<BTreeSet<u32>> {
        self.bitmaps.get(&handle).cloned()
    }

    fn agent_allowed(&self) -> BTreeSet<u32> {
        self.agent_allowed.clone()
    }

    fn live(&self) -> BTreeSet<u32> {
        self.live.clone()
    }

    fn universe(&self) -> BTreeSet<u32> {
        self.universe.clone()
    }
}
