#[macro_use]
extern crate pest_derive;

use pest::Parser;

#[derive(Parser)]
#[grammar = "vtt.pest"]
struct VttParser;

pub fn parse_file(contents: &str) -> Vec<Cue> {

}
