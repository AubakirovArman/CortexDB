use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalOperatorTrace {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub elapsed_nanos: u64,
}

pub trait PhysicalOp {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
    fn trace(&self) -> PhysicalOperatorTrace;
}

pub(crate) struct MaterializedOp<T> {
    items: Vec<Option<T>>,
    cursor: usize,
    trace: PhysicalOperatorTrace,
}

impl<T> MaterializedOp<T> {
    pub(crate) fn new(name: &str, input_count: usize, items: Vec<T>, elapsed_nanos: u64) -> Self {
        let output_count = items.len();
        Self {
            items: items.into_iter().map(Some).collect(),
            cursor: 0,
            trace: PhysicalOperatorTrace {
                name: name.to_owned(),
                input_count,
                output_count,
                elapsed_nanos,
            },
        }
    }
}

impl<T> PhysicalOp for MaterializedOp<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.items.get_mut(self.cursor).and_then(Option::take)?;
        self.cursor += 1;
        Some(value)
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        self.trace.clone()
    }
}

pub(crate) fn drain<O: PhysicalOp>(operator: &mut O) -> Vec<O::Item> {
    let mut values = Vec::new();
    while let Some(value) = operator.next() {
        values.push(value);
    }
    values
}

pub(crate) fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
