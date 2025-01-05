use pest::error::Error;
use pest::iterators::Pairs;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/html.pest"]
pub struct DjangoParser;


pub fn parse(input: &str)  -> Result<Pairs<Rule>, Error<Rule>> {
    let pairs = match DjangoParser::parse(Rule::html, input) {
        Ok(pairs) => pairs,
        Err(error) => return Err(error),
    };
    Ok(pairs)
}

