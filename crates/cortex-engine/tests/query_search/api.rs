use super::common::prelude::*;

#[test]
fn search_api_supports_keyword_and_vector_modes() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "alpha budget");
    indexes.add_vector(2, vec![0, 9]);

    let keyword = indexes.search(SearchQuery {
        text: "budget",
        vector: None,
        limit: 1,
        mode: SearchMode::Keyword,
    });
    assert_eq!(keyword[0].cell_id, 1);

    let vector = indexes.search(SearchQuery {
        text: "",
        vector: Some(&[0, 2]),
        limit: 1,
        mode: SearchMode::Vector,
    });
    assert_eq!(vector[0].cell_id, 2);
}
