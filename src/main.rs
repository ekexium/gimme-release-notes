use clap::Arg;
use curl::easy::{Easy, List};
use regex::Regex;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("{0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    String(String),
}

type Result<T> = std::result::Result<T, MyError>;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = clap::App::new("gimme release notes")
        .version("0.1.0")
        .arg(Arg::new("url").short('u').long("url").takes_value(true))
        .get_matches();
    let url = matches.value_of("url").unwrap();

    // get all commits
    let json = easy_get(url)?;
    let commits = json["commits"].as_array().unwrap();
    for commit in commits {
        let sha = commit["sha"].as_str().unwrap();
        let data = easy_get(&format!(
            "https://api.github.com/repos/tikv/tikv/commits/{}/pulls",
            sha
        ))?;
        let data = data.as_array().unwrap();
        if data.len() > 1 {
            return Err(MyError::String("too many pull requests".to_owned()));
        }
        if data.is_empty() {
            // direct commit on master, we don't care about them.
            continue;
        }
        let pr = data[0].as_object().unwrap();
        let labels = pr["labels"].as_array().unwrap();
        if labels
            .iter()
            .any(|label| label["name"].as_str().unwrap() == "release-note")
        {
            let number = pr["number"].as_u64().unwrap();
            let url = pr["html_url"].as_str().unwrap();
            println!("\n{}\n[#{}]({})", sha, number, url);
            let body = pr["body"].as_str().unwrap();
            let re = Regex::new(r#"### Release note[\s\S]*```([\s\S]*)```"#).unwrap();
            for cap in re.captures_iter(body) {
                println!("{}", &cap[1]);
            }
        }
    }
    Ok(())
}

fn easy_get(url: &str) -> Result<Value> {
    let mut easy = Easy::new();
    easy.url(url).unwrap();
    let mut list = List::new();
    list.append(&format!(
        "Authorization: bearer {}",
        std::env!("GITHUB_TOKEN")
    ))
    .unwrap();
    easy.http_headers(list).unwrap();
    easy.useragent("octocrab/0.1.0").unwrap();
    let mut dst = Vec::new();
    let mut transfer = easy.transfer();
    transfer
        .write_function(|data| {
            dst.extend_from_slice(data);
            Ok(data.len())
        })
        .unwrap();
    transfer.perform().unwrap();
    drop(transfer);
    let data = String::from_utf8(dst).unwrap();
    Ok(serde_json::from_str(&data)?)
}
