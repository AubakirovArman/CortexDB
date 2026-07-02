#[path = "fail_closed_invariant_model/fixture.rs"]
mod fixture;

#[test]
fn bitmap_program_respects_fail_closed_model_over_randomized_catalog_views() {
    fixture::assert_bitmap_program_model();
}
