use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    compute_bitmap_stack_depth, eval_bitmap_program, eval_bitmap_program_bitmap, BitmapHandle,
    BitmapOp, BitmapProgram, BitmapProvider, BitmapVmError, MockBitmapProvider, RoaringBitmap,
};

fn set(values: &[u32]) -> BTreeSet<u32> {
    values.iter().copied().collect()
}

fn roaring(values: impl IntoIterator<Item = u32>) -> RoaringBitmap {
    values.into_iter().collect()
}

fn program(ops: Vec<BitmapOp>) -> BitmapProgram {
    let max_stack_depth = compute_bitmap_stack_depth(&ops).unwrap();
    BitmapProgram {
        ops,
        max_stack_depth,
    }
}

struct DirectRoaringProvider {
    bitmaps: BTreeMap<BitmapHandle, RoaringBitmap>,
    universe: RoaringBitmap,
}

impl BitmapProvider for DirectRoaringProvider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<RoaringBitmap> {
        self.bitmaps.get(&handle).cloned()
    }

    fn agent_allowed(&self) -> RoaringBitmap {
        self.universe.clone()
    }

    fn live(&self) -> RoaringBitmap {
        self.universe.clone()
    }

    fn universe(&self) -> RoaringBitmap {
        self.universe.clone()
    }
}

#[test]
fn agent_allowed_live_scope_status_returns_expected_set() {
    let provider = MockBitmapProvider {
        bitmaps: BTreeMap::from([
            (BitmapHandle(10), set(&[2, 3, 5])),
            (BitmapHandle(20), set(&[3, 4, 5])),
        ]),
        agent_allowed: set(&[1, 2, 3, 4]),
        live: set(&[2, 3, 4, 5]),
        universe: set(&[1, 2, 3, 4, 5]),
    };
    let program = program(vec![
        BitmapOp::PushAgentAllowed,
        BitmapOp::PushLive,
        BitmapOp::And,
        BitmapOp::Push(BitmapHandle(10)),
        BitmapOp::And,
        BitmapOp::Push(BitmapHandle(20)),
        BitmapOp::And,
    ]);
    assert_eq!(eval_bitmap_program(&program, &provider).unwrap(), set(&[3]));
}

#[test]
fn roaring_vm_handles_large_dense_bitmap_without_btreeset_stack() {
    let provider = DirectRoaringProvider {
        bitmaps: BTreeMap::from([
            (BitmapHandle(10), roaring((0_u32..1_000_000).step_by(2))),
            (BitmapHandle(20), roaring((0_u32..1_000_000).step_by(3))),
        ]),
        universe: roaring(0_u32..1_000_000),
    };
    let program = program(vec![
        BitmapOp::Push(BitmapHandle(10)),
        BitmapOp::Push(BitmapHandle(20)),
        BitmapOp::And,
    ]);

    let result = eval_bitmap_program_bitmap(&program, &provider).unwrap();

    assert_eq!(result.len(), 166_667);
    assert!(result.contains(999_996));
    assert!(!result.contains(999_998));
}

#[test]
fn not_works_as_universe_complement() {
    let provider = MockBitmapProvider {
        bitmaps: BTreeMap::from([(BitmapHandle(10), set(&[2]))]),
        agent_allowed: BTreeSet::new(),
        live: BTreeSet::new(),
        universe: set(&[1, 2, 3]),
    };
    let program = program(vec![BitmapOp::Push(BitmapHandle(10)), BitmapOp::Not]);
    assert_eq!(
        eval_bitmap_program(&program, &provider).unwrap(),
        set(&[1, 3])
    );
}

#[test]
fn push_universe_loads_segment_universe() {
    let provider = MockBitmapProvider {
        universe: set(&[1, 2, 3]),
        ..MockBitmapProvider::default()
    };
    let program = program(vec![BitmapOp::PushUniverse]);
    assert_eq!(
        eval_bitmap_program(&program, &provider).unwrap(),
        set(&[1, 2, 3])
    );
}

#[test]
fn invalid_stack_program_fails() {
    let provider = MockBitmapProvider::default();
    let program = BitmapProgram {
        ops: vec![BitmapOp::And],
        max_stack_depth: 0,
    };
    assert_eq!(
        eval_bitmap_program(&program, &provider).unwrap_err(),
        BitmapVmError::StackUnderflow
    );
}
