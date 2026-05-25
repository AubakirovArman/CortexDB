use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionFragment {
    pub tag: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct CellAccumulator {
    sections: BTreeMap<u16, Vec<u8>>,
}

impl CellAccumulator {
    pub fn push(&mut self, fragment: SectionFragment) {
        self.sections.insert(fragment.tag, fragment.data);
    }

    pub fn finish(self) -> Vec<SectionFragment> {
        self.sections
            .into_iter()
            .map(|(tag, data)| SectionFragment { tag, data })
            .collect()
    }
}
