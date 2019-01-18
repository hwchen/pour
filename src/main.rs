use failure::{Error, format_err};
use reqwest::{Request, Url};
use std::fs;
use std::iter;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::thread;
use structopt::StructOpt;

fn main() -> Result<(), Error> {
    let opt = CliOpt::from_args();

    let timeout_sec = opt.timeout;

    // first build setlist
    // TODO make url conflict with file
    let req_list: Vec<_> = if let Some(f) = opt.file {
        let buf = fs::read_to_string(f)?;

        let url_set = buf.lines()
            .map(|line| {
                line.parse::<Url>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        let urls = iter::repeat(url_set.clone()).take(opt.n)
            .flat_map(|url_set| url_set.into_iter())
            .map(|url| {
                reqwest::Client::new()
                    .get(url)
                    .build()
                    .expect("Failed building request")
            })
            .collect();

        urls
    } else {
        let url = opt.url.ok_or_else(||format_err!("url to test is required"))?;
        let url: Url = url.parse()?;

        let urls = iter::repeat(url).take(opt.n);
        urls.map(|url| {
            reqwest::Client::new()
                .get(url)
                //.basic_auth(user_pass[0], Some(user_pass[1]))
                .build()
                .expect("Failed building request")
            })
            .collect()
    };

    // Now perform calls
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_sec * 1000))
        .build()
        .expect("Could not build client");


    if opt.asynchronous {
        let mut handles = vec![];
        for req in req_list {
            let client = client.clone();
            let handle = thread::spawn(move || {
                request(&client, req).expect("request failed");
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("error joining");
        }
    } else {
        for req in req_list {
            request(&client, req)?;
        }
    }

    Ok(())
}

fn request(client: &reqwest::Client, req: Request) -> Result<(), Error> {

    let profile_url = req.url().to_owned();

    let start = Instant::now();
    let mut res = client.execute(req).expect("Could not send req");
    let end = start.elapsed();

    println!("{}.{:03}s, {}, {}",end.as_secs(), end.subsec_millis(), res.status(), profile_url);
    if res.status().is_server_error() {
        println!("{}", res.text()?)
    }
    // if verbose, print the page.
    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(name="pour")]
struct CliOpt {
    #[structopt(name="verbose", short="v", long="verbose")]
    verbose: bool,

    #[structopt(name="repetitions", short="n", default_value="10", help="repetitions of request")]
    n: usize,

    #[structopt(name="timeout", short="t", default_value="1000", help="timeout in s")]
    timeout: u64,

    #[structopt(name="url", long="url")]
    //#[structopt(global=true)]
    url: Option<String>,

    #[structopt(name="file", short="f", long="file")]
    #[structopt(parse(from_os_str))]
    #[structopt(global=true)]
    file: Option<PathBuf>,

    #[structopt(name="async", short="a", long="async")]
    #[structopt(global=true)]
    asynchronous: bool,
}
