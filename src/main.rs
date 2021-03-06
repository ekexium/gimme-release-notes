use clap::Arg;
use curl::easy::{Easy, List};
use indicatif::ProgressBar;
use regex::Regex;
use serde_json::Value;
use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("{0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    String(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, MyError>;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = clap::App::new("gimme release notes")
        .version("0.1.0")
        .arg(
            Arg::new("repo")
                .short('r')
                .long("repo")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("range")
                .long("range")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .takes_value(true)
                .required(true),
        )
        .get_matches();
    let repo = matches.value_of("repo").unwrap();
    let range = matches.value_of("range").unwrap();
    let output = matches.value_of("output").unwrap();

    // get all commits
    let page_size = 30;
    let mut page_id = 1;
    let mut handles = vec![];
    let json = easy_get(&format!(
        "https://api.github.com/repos/{}/compare/{}?page={}&page_size={}",
        repo, range, page_id, page_size
    ))?;
    let total_commit = json["total_commits"].as_u64().unwrap();
    let bar = Arc::new(ProgressBar::new(total_commit));

    loop {
        let repo = repo.to_owned();
        let range = range.to_owned();
        let bar = bar.clone();
        let handle = tokio::spawn(async move {
            let json = easy_get(&format!(
                "https://api.github.com/repos/{}/compare/{}?page={}&page_size={}",
                repo, range, page_id, page_size
            ))?;
            let commits = json["commits"].as_array().unwrap();
            handle_a_batch(commits, &repo, bar)
        });
        handles.push(handle);
        if page_id * page_size >= total_commit {
            break;
        }
        page_id += 1;
    }
    println!("all workers started");

    let mut out_file: File = std::fs::File::create(output)?;
    for handle in handles {
        let s = handle.await.unwrap()?;
        out_file.write_all(s.as_bytes())?;
    }
    bar.finish();
    Ok(())
}

fn handle_a_batch(commits: &Vec<Value>, repo: &str, bar: Arc<ProgressBar>) -> Result<String> {
    let mut res = String::new();
    for commit in commits {
        let sha = commit["sha"].as_str().unwrap();
        bar.inc(1);
        let data = easy_get(&format!(
            "https://api.github.com/repos/{}/commits/{}/pulls",
            repo, sha
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
            write!(res, "\n{}\n[#{}]({})", sha, number, url).unwrap();
            let body = pr["body"].as_str().unwrap();
            let re = Regex::new(r#"### Release note[\s\S]*```([\s\S]*)```"#).unwrap();
            for cap in re.captures_iter(body) {
                write!(res, "{}", &cap[1]).unwrap();
            }
        }
    }
    Ok(res)
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
