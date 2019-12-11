use anyhow::{Context, Error};
use hyper::{
    client::connect::Connection,
    service::Service,
    Client,
    Request,
    Uri,
};
use hyper_tls::HttpsConnector;
use std::{
    fs,
    iter,
    path::PathBuf,
    time::Instant,
};
use structopt::StructOpt;
use tokio::io::{AsyncRead, AsyncWrite};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opt = CliOpt::from_args();

    if opt.n == 0 {
        panic!("Cannot repeat 0 times");
    }

    let _timeout_sec = opt.timeout;

    // Build Client
    let https = HttpsConnector::new();
    let client = Client::builder()
        .build::<_, hyper::Body>(https);

    // first build setlist
    // TODO make url conflict with file
    let url_set: Vec<_> = if let Some(f) = opt.file {
        let buf = fs::read_to_string(&f)
            .with_context(|| format!("Error reading config file {:?}", f))?;

        buf.lines()
            .map(|line| {
                line.parse::<Uri>()
                    .with_context(|| format!("Error parsing url line {:?}", line))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let url = opt.url.context("Missing url to test")?;
        let url: Uri = url.parse()
            .with_context(|| format!("Error parsing url {:?}", url))?;

        vec![url]
    };

    let url_set_len = url_set.len();

    let req_list = iter::repeat(url_set).take(opt.n)
        .flat_map(|url_set| url_set.into_iter())
        .map(|url| {
            Request::get(url)
                .body(hyper::Body::default())
                .expect("Failed building request")
        });

    // perform calls
    if opt.asynchronous {
        let tasks = url_set_len * opt.n;
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
                .await?;
        }
    }

    Ok(())
}

async fn exec_request<C>(client: &Client<C, hyper::Body>, req: Request<hyper::Body>) -> Result<(), hyper::Error>
    where C: Service<hyper::Uri> + Clone + Send + Sync + 'static,
        // this is the one that gets around sealed Connect
        C::Response: Connection + AsyncRead + AsyncWrite + Send + Unpin + 'static,
        C::Future: Send + Unpin + 'static,
        C::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>
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

