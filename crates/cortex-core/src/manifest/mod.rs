#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SegmentId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentHandle {
    pub id: SegmentId,
    pub generation: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub generation: u64,
    pub live_segments: Vec<SegmentHandle>,
    pub retired_segments: Vec<SegmentHandle>,
}

impl Manifest {
    pub fn add_live_segment(&mut self, handle: SegmentHandle) {
        self.generation += 1;
        self.live_segments.push(handle);
    }

    pub fn retire_segment(&mut self, id: SegmentId) {
        if let Some(index) = self.live_segments.iter().position(|handle| handle.id == id) {
            self.generation += 1;
            self.retired_segments.push(self.live_segments.remove(index));
        }
    }
}
