/*!
Wikipedia page scraper for fetching all relevant links and connected wiki pages.
The script uses relative wiki paths, as connected page links are also relative.

Usage :

    - ./target/release/wikipedia_links "/wiki/Rust_(programming_language)"
*/

use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use std::env;
use std::error::Error;

// Base wikipedia url used for constructing the actual url to a particular wiki page
// Is required because internal wiki links would have relative path common to this domain
static BASE_WIKI_URL: &str = "https://en.wikipedia.org";

// Wikipedia reguar expression which can be used to match and filter wiki links
static WIKI_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("^/wiki/(?P<name>[^\"]+)$").expect("To be a valid regex"));

/**
Utility function that is used to fetch the wiki page html document and filter out the relevant links.
The argument `url_ref` is the relative path ( excluding the wiki domain ) to the page.
*/
fn fetch_wiki_links(url_ref: &str) -> Result<Vec<String>, Box<dyn Error>> {
    assert!(
        WIKI_REGEX.is_match(url_ref),
        "Should be a valid Wikipedia page ref"
    );

    // Fetch the wikipedia html page content and construct the document
    let html = reqwest::blocking::get(format!("{BASE_WIKI_URL}{url_ref}"))?.text()?;
    let document = Html::parse_document(&html);

    // Find the main content div
    let content_div = document
        .select(&Selector::parse("div#bodyContent")?)
        .next()
        .expect("Should have the main content div");

    // All the relevant content is inside the paragraphs
    // and so we fetch all the anchors from it and extract their 'href' links
    let links = content_div
        .select(&Selector::parse("p a[href]")?)
        .map(|link_tag| {
            link_tag
                .value()
                .attr("href")
                .expect("Every anchor tag must have a 'href' attribute")
                .to_string()
        })
        .filter(|link| WIKI_REGEX.is_match(link))
        .collect();

    Ok(links)
}

fn main() -> Result<(), Box<dyn Error>> {
    let wiki_url_ref = env::args()
        .nth(1)
        .expect("Relative wiki page path must be provided");

    let link_records = fetch_wiki_links(&wiki_url_ref)?;
    println!("{:?}", link_records);

    Ok(())
}
