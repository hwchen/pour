use hyper::{Client, Request, Uri};
use hyper_tls::HttpsConnector;
use snafu::{Snafu, ResultExt, OptionExt};
use std::fs;
use std::iter;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opt = CliOpt::from_args();

    let timeout_sec = opt.timeout;
        //.timeout(Duration::from_millis(timeout_sec * 1000))

    // Build Client
    let https = HttpsConnector::new().expect("Failed to build https connector");
    let client = Client::builder()
        .build::<_, hyper::Body>(https);

    // first build setlist
    // TODO make url conflict with file
    let req_list: Vec<_> = if let Some(f) = opt.file {
        let buf = fs::read_to_string(&f)
            .context(ReadConfigFile { path: f })?;

        let url_set = buf.lines()
            .map(|line| {
                line.parse::<Uri>()
                    .context(ParseUrl { input: line })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let urls = iter::repeat(url_set.clone()).take(opt.n)
            .flat_map(|url_set| url_set.into_iter())
            .map(|url| {
                Request::get(url)
                    .body(hyper::Body::default())
                    .expect("Failed building request")
            })
            .collect();

        urls
    } else {
        let url = opt.url.context(MissingUrl)?;
        let url: Uri = url.parse()
            .context(ParseUrl { input: url })?;

        let urls = iter::repeat(url).take(opt.n);
        urls.map(|url| {
                Request::get(url)
                //.basic_auth(user_pass[0], Some(user_pass[1]))
                .body(hyper::Body::default())
                .expect("Failed building request")
            })
            .collect()
    };

    // perform calls
    if opt.asynchronous {
        let tasks = req_list.len();
        let (tx, mut rx) = tokio::sync::mpsc::channel(tasks);

        for req in req_list {
            // doesn't use connection pool, just opens a ton of sockets.
            let client = client.clone();
            let mut tx = tx.clone();
            tokio::spawn(async move {
                exec_request(&client, req).await
                    .expect("Failed to execute request");
                let _ = tx.send(1).await;
            });
        }

        let mut completed = 0usize;
        while completed < tasks {
            match rx.recv().await {
                None => panic!("Failed to complete request"),
                Some(v) => {
                    completed += v;
                }
            }
        }
    } else {
        for req in req_list {
            exec_request(&client, req)
                .await
                .context(RequestExec)?;
        }
    }

    Ok(())
}

async fn exec_request<C>(client: &Client<C, hyper::Body>, req: Request<hyper::Body>) -> Result<(), hyper::Error>
    where C: hyper::client::connect::Connect + 'static
{
    let profile_url = req.uri().to_owned();

    let start = Instant::now();
    let res = client.request(req).await?;
    let end = start.elapsed();

    println!("{}.{:03}s, {}, {}",end.as_secs(), end.subsec_millis(), res.status(), profile_url);
    if res.status().is_server_error() {
        println!("{}", res.status())
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


#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Error reading config file {}: {}", path.display(), source))]
    ReadConfigFile { source: std::io::Error, path: PathBuf},

    #[snafu(display("Missing url to test"))]
    MissingUrl,

    #[snafu(display("Error parsing url {}: {}", input, source))]
    ParseUrl { source: http::uri::InvalidUri, input: String },

    #[snafu(display("Error executing request: {}", source))]
    RequestExec { source: hyper::Error },
}
