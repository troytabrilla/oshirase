mod error;

pub use error::AggregatorError;

/* TODO
* Big refactor for aggregator from full-ish data pipeline to simpler cli util
* Aggregator should contain a single lib.rs and main.rs, and maybe helper files, but otherwise pretty flat
* Should implement only the transform/aggregator part
* Needs to combine main list with alt titles and extras
* Alt titles need to be combined first
* Extras can vary, so need to standardize the data structure for them
* Use https://crates.io/crates/rayon and https://crates.io/crates/levenshtein_automata to speed up performance
* Read json from stdin and write json to stdout, will be called from Go API project
* Read config parameters from cli args, create a default config struct and override defaults with cli args
* fn add_alt_titles(list, alt_titles) -> list; use media_id for joins
* fn add_extras(list, extras) -> list called for variable number of extras; try title hash, then title levanshtein distance for all title combinations
* Add best non-exact matches to alt titles and persist for future runs, loading alt titles should provide exact match on next run
*/

pub fn run() -> Result<(), AggregatorError> {
    Ok(())
}
