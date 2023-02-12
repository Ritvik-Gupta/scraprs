use chrono::Utc;
use std::path::Path;
use thirtyfour::{
    prelude::{ElementQueryable, ElementWaitable, WebDriverResult},
    By, ChromeCapabilities, WebDriver, WebElement,
};

static LEETCODE_DOMAIN: &str = "https://leetcode.com";

lazy_static::lazy_static! {
    static ref OUTPUT_PATH: String = {
        let path = std::env::args().collect::<Vec<_>>()[1].clone();
        Path::new(&path).exists().then(|| path).expect("for output path to exist")
    };
}

#[derive(serde::Serialize)]
struct PotdInfo {
    number: u32,
    name: String,
    url: String,
    solution_url: Option<String>,
}

/// POTD scraping function. Returns PotdInfo if it was successful in scraping.
async fn scrape_potd(potd: WebElement) -> WebDriverResult<PotdInfo> {
    let problem_anchor = potd
        .find(By::Css("div[role='cell']:nth-child(2) a"))
        .await?;

    // Problem anchor's content would be Title ( Problem Number + Name )
    let name = problem_anchor.inner_html().await?;
    let (number, name) = name
        .split_once(". ")
        .expect("is of leetcode problem name format");
    let number = number.parse::<u32>().expect("is a leetcode problem number");

    let url = format!(
        "{LEETCODE_DOMAIN}{}",
        problem_anchor
            .attr("href")
            .await?
            .expect("to have an href attribute")
    );

    let solution_cell = potd.find(By::Css("div[role='cell']:nth-child(3)")).await?;

    // A solution anchor might not be present ( no solution exists )
    let solution_anchor = solution_cell
        .query(By::Css("a[aria-label='solution']"))
        .first_opt()
        .await?;

    let solution_url = match solution_anchor {
        Some(solution_anchor) => Some(format!(
            "{LEETCODE_DOMAIN}{}",
            solution_anchor
                .attr("href")
                .await?
                .expect("to have an href attribute")
        )),
        None => None,
    };

    Ok(PotdInfo {
        number,
        name: name.to_string(),
        url,
        solution_url,
    })
}

/// Main Scraping function.
async fn scraping(driver: &WebDriver) -> WebDriverResult<()> {
    driver
        .goto(format!("{LEETCODE_DOMAIN}/problemset/all/"))
        .await?;

    // Wait for body to hydrate loading screen
    let body = driver.find(By::Tag("body")).await?;
    body.wait_until()
        .condition(Box::new(|body| {
            Box::pin(async move { Ok(!body.class_name().await?.unwrap_or_default().is_empty()) })
        }))
        .await?;

    let table = driver
        .find(By::Css("div:has(div[role='table']).pointer-events-none"))
        .await;

    let table = match table {
        // Wait till Table is active and problems are loaded
        Ok(table) => {
            table
                .wait_until()
                .condition(Box::new(|table| {
                    Box::pin(async move {
                        let classes = table.class_name().await?.unwrap_or_default();
                        Ok(!classes.contains("pointer-events-none"))
                    })
                }))
                .await?;

            table
        }
        // Unlikely case when Table has been loaded before query
        Err(_) => driver.find(By::Css("div:has(div[role='table'])")).await?,
    };

    assert!(!table
        .class_name()
        .await?
        .unwrap_or_default()
        .contains("pointer-events-none"));

    let table = table.find(By::Css("div[role='rowgroup']")).await?;

    // Wait for Table to load POTD
    table
        .wait_until()
        .condition(Box::new(|table| {
            Box::pin(async move {
                let first_problem = table.find(By::Css("div[role='row']")).await?;

                // POTD would have a unique SVG element
                Ok(first_problem
                    .query(By::Css("div[role='cell'] > a > svg"))
                    .exists()
                    .await?)
            })
        }))
        .await?;

    let first_problem = table.find(By::Css("div[role='row']")).await?;

    assert!(
        first_problem
            .query(By::Css("div[role='cell'] > a > svg"))
            .exists()
            .await?
    );

    let potd_info = scrape_potd(first_problem).await?;

    let mut toml_table = toml::to_string_pretty(&potd_info)
        .expect("to serialize properly")
        .parse::<toml::Table>()
        .unwrap();
    toml_table.insert(
        "date".to_string(),
        toml::Value::String(Utc::now().format("%Y%m%d").to_string()),
    );

    // Create TOML structure for PotdInfo
    let toml_table =
        toml::Table::from_iter([("potd".to_string(), toml::Value::Table(toml_table))].into_iter());

    std::fs::write(OUTPUT_PATH.to_string(), toml_table.to_string())
        .expect("to write to toml file successfully");

    Ok(())
}

#[tokio::main]
async fn main() -> WebDriverResult<()> {
    let mut caps = ChromeCapabilities::new();
    caps.set_headless()?;

    let driver = WebDriver::new("http://localhost:9515", caps).await?;

    let res = scraping(&driver).await;
    // Perform a Quit operation even if scraping fails
    driver.quit().await?;

    res
}
