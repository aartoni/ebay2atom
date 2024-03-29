use core::fmt::Write;
use std::{
    io::{self, Read},
    time::SystemTime,
};

use atom_syndication::{
    Content, Entry, FeedBuilder, GeneratorBuilder, LinkBuilder, TextBuilder, TextType, WriteConfig,
};
use chrono::{DateTime, Local};
use regex::Regex;
use scraper::{Html, Selector};

// Manifest environment variables
const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const NAME: &str = env!("CARGO_PKG_NAME");

// eBay-specific constants
const EBAY_SEARCH_RESULTS: usize = 71;
const FEED_TITLE_QUERY: &str = r#"input[name="_nkw"]"#;
const ITEMS_QUERY: &str = ".srp-river .srp-river-results .s-item__wrapper";
const TITLE_QUERY: &str = ".s-item__title span[role=heading]";
const LINK_QUERY: &str = ".s-item__link";
const PRICE_QUERY: &str = ".s-item__price";
const CONDITION_QUERY: &str = ".SECONDARY_INFO";
const TIME_LEFT_QUERY: &str = ".s-item__time-left";
const PURCHASE_OPTIONS_QUERY: &str = ".s-item__purchase-options";
const AD_QUERY: &str = ".lvformat";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get document
    let mut html = String::new();
    io::stdin().read_to_string(&mut html)?;
    let document = Html::parse_document(&html);

    // Get feed data
    let feed_title_selector = Selector::parse(FEED_TITLE_QUERY)?;
    let feed_title_input = document.select(&feed_title_selector).next().unwrap();
    let feed_title = feed_title_input.value().attr("value").unwrap();

    // Get feed link
    let link_regex = Regex::new(r#"baseUrl":"(https://[^&]+).*?""#)?;
    let feed_link = link_regex.captures(&html).unwrap().get(1).unwrap().as_str();

    // Get generator
    let generator = GeneratorBuilder::default()
        .uri(Some(REPOSITORY.to_owned()))
        .version(Some(VERSION.to_owned()))
        .value(NAME.to_owned())
        .build();

    // Get links
    let feed_link = LinkBuilder::default()
        .rel("alternate".to_owned())
        .mime_type(Some("text/html".to_owned()))
        .href(feed_link.to_owned())
        .build();

    // Get title
    let feed_title = TextBuilder::default()
        .r#type(TextType::Text)
        .value(feed_title.to_owned())
        .build();

    // Get local DateTime
    let update_time: DateTime<Local> = SystemTime::now().into();

    // Build feed (except entries)
    let mut feed = FeedBuilder::default()
        .generator(Some(generator))
        .links(vec![feed_link])
        .title(feed_title)
        .updated(update_time)
        .build();

    // Get item selectors and regexes
    let title_selector = Selector::parse(TITLE_QUERY)?;
    let link_selector = Selector::parse(LINK_QUERY)?;
    let price_selector = Selector::parse(PRICE_QUERY)?;
    let condition_selector = Selector::parse(CONDITION_QUERY)?;
    let time_left_selector = Selector::parse(TIME_LEFT_QUERY)?;
    let purchase_options_selector = Selector::parse(PURCHASE_OPTIONS_QUERY)?;
    let ad_selector = Selector::parse(AD_QUERY)?;
    let items_selector = Selector::parse(ITEMS_QUERY)?;
    let url_regex = Regex::new(r"https.+(\d{10})")?;

    // Store the entries array
    let mut entries: Vec<Entry> = Vec::with_capacity(EBAY_SEARCH_RESULTS);

    // Parse feed items
    for item in document.select(&items_selector) {
        let mut entry = Entry::default();
        let mut content = Content::default();
        content.set_content_type(Some("xhtml".to_owned()));
        let mut description = r#"<div xmlns="http://www.w3.org/1999/xhtml">"#.to_owned();

        // Get title
        let title = item
            .select(&title_selector)
            .next()
            .unwrap()
            .text()
            .last()
            .unwrap();

        entry.set_title(title);

        // Get item link/id
        let item_url = item
            .select(&link_selector)
            .next()
            .unwrap()
            .value()
            .attr("href")
            .unwrap();

        let url_captures = url_regex.captures(item_url).unwrap();
        let item_url = url_captures.get(0).unwrap().as_str();

        let link = LinkBuilder::default()
            .rel("alternate".to_owned())
            .mime_type(Some("text/html".to_owned()))
            .href(item_url.to_owned())
            .build();

        entry.set_links([link]);
        entry.set_id(item_url);

        // Get price
        let price = item
            .select(&price_selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap();

        write!(description, "<p>Price: {price}</p>")?;

        // Get condition
        if let Some(condition) = item.select(&condition_selector).next() {
            let condition = condition.text().next().unwrap();
            write!(description, "<p>Condition: {condition}</p>")?;
        }

        // Get time left
        if let Some(time_left) = item.select(&time_left_selector).next() {
            let time_left = time_left.text().next().unwrap();
            write!(description, "<p>Time left: {time_left}</p>")?;
        }

        // Get purchase options
        if let Some(purchase_options) = item.select(&purchase_options_selector).next() {
            let purchase_options = purchase_options.text().next().unwrap();
            write!(description, "<p>Purchase options: {purchase_options}</p>")?;
        }

        // Get ad
        if let Some(ad) = item.select(&ad_selector).next() {
            let ad = ad.text().next().unwrap();
            write!(description, "<p>Ad: {ad}</p>")?;
        }

        // Finish and append entry
        description.push_str("</div>");
        content.set_value(description);
        entry.set_content(content);
        entry.set_updated(update_time);
        entries.push(entry);
    }

    feed.set_entries(entries);

    let write_config = WriteConfig {
        write_document_declaration: true,
        indent_size: Some(2),
    };

    feed.write_with_config(io::stdout(), write_config)?;
    Ok(())
}
