use std::collections::HashSet;

use reqwest;
use scraper::{Html, Selector};

use crate::app_config::WEBSITE_MAX_CHARACTER_LENGTH;

pub async fn scrape_site_and_get_text(website_url: &str) -> anyhow::Result<String> {
    // URL to scrape

    // 1) Fetch the page contents as a String
    let response_text = reqwest::get(website_url).await?.text().await?;

    // 2) Parse the HTML
    let document = Html::parse_document(&response_text);

    // Selectors for common "textual" elements
    // (p, h1–h6, span, li, etc.). Adjust to your needs!
    let text_selector = Selector::parse(
        "
        p,
        h1, h2, h3, h4, h5, h6,
        li,
        span,
        div,
        a,
        blockquote,
        section,
        article,
        main
    ",
    )
    .unwrap();

    // 4) Extract all text from the <body> (or entire document)
    let raw_text = document
        .select(&text_selector) // Find all <body> elements
        .flat_map(|body| body.text()) // Get text from within <body>
        .collect::<Vec<_>>() // Collect into a vec of &str
        .join(" "); // Join with spaces

    let body_text = raw_text
        .lines()
        .filter(|line| {
            // A simple check: skip lines that contain obvious JS tokens
            !(line.contains("function")
                || line.contains("var ")
                || line.contains("if (")
                || line.contains("if(")
                || line.contains("then(")
                || line.contains("} else")
                || line.contains("else {")
                || line.contains("for(")
                || line.contains("for (")
                || line.contains("i=")
                || line.contains("{")
                || line.contains("}")
                || line.contains("()")
                || line.contains("]")
                || line.contains("[")
                || line.contains("[i]")
                || line.ends_with(";")
                || line.contains("// ")
                || line.contains("/*")
                || line.contains("*/")
                || line.contains("-wrapper")
                || line.contains("webkit")
                || line.contains(":hover")
                || line.contains("const ")
                || line.contains("+ i")
                || line.contains("console.")
                || line.contains("document.")
                || line.contains("$("))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let final_text = body_text
        .lines() // split by ''
        .map(str::trim) // trim each line
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let final_text = dedup_text(final_text);

    let final_text = first_n_chars(&final_text, WEBSITE_MAX_CHARACTER_LENGTH as usize);
    // println!("{}", final_text);

    Ok(final_text)
}

fn dedup_text(text: String) -> String {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for line in text.lines() {
        if !seen.contains(line) {
            seen.insert(line);
            deduped.push(line);
        }
    }

    let final_text = deduped.join("\n");
    final_text
}

// grap first N charcters
pub fn first_n_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect::<String>()
}
