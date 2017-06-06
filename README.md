# Monitoring A Rust Web Server with Dogstatsd-rs

## Introduction

Systems programming language [Rust](https://www.rust-lang.org/en-US/) is gaining more widespread use [in production](https://www.rust-lang.org/en-US/friends.html). A [stable Rust Dogstatsd client also now exists](https://github.com/mcasper/dogstatsd-rs), allowing the monitoring of custom metrics for Rust applications in Datadog. Finally, Rust has been [developing its web ecosystem](http://www.arewewebyet.org) to reach parity with other major languages.

In this tutorial, we'll draw on these new developments to build a Rust web application using the [Rocket server framework](https://rocket.rs). Then we'll integrate our app with the Dogstatsd client to report metrics. Finally, we'll take a look at the graphs of our metrics in Datadog.

## Part 0: Dependencies

You'll need Datadog, Rust, and SQLite installed before proceeding.

Directions for installing and setting up the Datadog web agent can be found [here](http://docs.datadoghq.com/guides/basic_agent_usage/).

To install Rust on your system, run

```
curl https://sh.rustup.rs -sSf | sh
```

in your terminal and follow the instructions. This command will also set up Rust's package manager, [Cargo](http://doc.crates.io/guide.html), which we will be using later to run our Rust program.

After installing Rust, change your system-wide default Rust version to [nightly](https://doc.rust-lang.org/book/nightly-rust.html) by running `rustup default nightly`. This is necessary because Rocket uses nightly features of Rust for code generation. (Should you wish to use stable or beta Rust instead, run `rustup default stable`
or `rustup default beta` after completing this tutorial).

Our Rocket web app will interact with a SQLite database using the [rusqlite](https://github.com/jgallagher/rusqlite) library, which requires SQLite version 3.6.8 or higher. SQLite can be downloaded [from the project website](https://sqlite.org/download.html) or via Linux package manager following general package management guidelines specific to your Linux distribution.

## Part 1: Creating A Web Server with Rocket

First, let's create our Rust web server.

The directory structure of your Rust application will look like this:

```
├── Cargo.lock
├── Cargo.toml
└── src
    └── main.rs
```

Let's quickly run through what these files do in case you're unfamiliar. Cargo.lock, which contains information regarding dependencies, will appear once you compile and run your program for the first time. It is auto-generated and should not be modified. Cargo.toml is called the manifest and is where you will explicitly add relevant project metadata, including the libraries you will be using, the name of your project, and the location of binaries to compile. `src/main.rs` contains the program itself.

Here's what our `Cargo.toml` will look like. Note that we specify the most recent version of the Rocket library (0.2.8), as the Rocket project is frequently changing.

<script src="https://gist.github.com/outkaj/403b6230efd7e31f6ab21586f5c9ff67.js"></script>

Now let's take a look at the main application logic. Our Rocket web app will create a SQLite database, store the entry "Datadog" in the database, and display that entry at `http://localhost:8000` when run.

Here's the `main.rs`:

<script src="https://gist.github.com/outkaj/4663f4e9791bceba1498adb80845992f.js"></script>

We can use the `cargo run` command to compile and run our application.

If all goes well, the terminal output should look like this, with the filepath after "Running"
being specific to your system.

```
   Compiling typeable v0.1.2
   Compiling httparse v1.2.3
   Compiling num-traits v0.1.37
   Compiling matches v0.1.4
   Compiling pkg-config v0.3.9
   Compiling log v0.3.8
   Compiling term v0.4.5
   Compiling unicode-normalization v0.1.4
   Compiling toml v0.2.1
   Compiling libsqlite3-sys v0.8.1
   Compiling byteorder v1.0.0
   Compiling traitobject v0.1.0
   Compiling ansi_term v0.9.0
   Compiling base64 v0.5.2
   Compiling num-integer v0.1.34
   Compiling num-iter v0.1.33
   Compiling num v0.1.37
   Compiling libc v0.2.23
   Compiling semver v0.1.20
   Compiling term-painter v0.2.3
   Compiling linked-hash-map v0.4.2
   Compiling state v0.2.1
   Compiling lru-cache v0.1.1
   Compiling num_cpus v1.5.0
   Compiling bitflags v0.9.1
   Compiling itoa v0.3.1
   Compiling memchr v1.0.1
   Compiling mime v0.2.6
   Compiling rustc_version v0.1.7
   Compiling time v0.1.37
   Compiling version_check v0.1.0
   Compiling dtoa v0.4.1
   Compiling unicase v1.4.0
   Compiling hyper v0.10.11
   Compiling rocket_codegen v0.2.8
   Compiling rocket v0.2.8
   Compiling chrono v0.2.25
   Compiling rusqlite v0.12.0
   Compiling language-tags v0.2.2
   Compiling unicode-bidi v0.3.3
   Compiling serde v0.9.15
   Compiling idna v0.1.2
   Compiling dogstatsd v0.1.1
   Compiling url v1.4.1
   Compiling cookie v0.6.2
   Compiling serde_json v0.9.10
   Compiling rocket_contrib v0.2.8
   Compiling rocket_dogstatsd_example v0.0.1 (file:///home/petrova/Projects/Rust/rocket_dogstatsd_example)
   Finished dev [unoptimized + debuginfo] target(s) in 68.87 secs
   Running `/tmp/cargo/misc/debug/main`
🔧  Configured for development.
    => address: localhost
    => port: 8000
    => log: normal
    => workers: 4
🛰  Mounting '/':
    => GET /
🚀  Rocket has launched from http://localhost:8000...
```

Now navigate to `http://localhost:8000`, which should say "Datadog".

## Part 2: Reporting Server Metrics with Dogstatsd-rs

Now we want to report metrics regarding our web server to Datadog with the dogstatsd-rs library.

First, we add the library to our `Cargo.toml`:

`dogstatsd = "0.1"`

We'll use dogstatsd-rs to send a counter and some histograms to Datadog. These, along with other DogStatsD metrics, are described further in the [docs](http://docs.datadoghq.com/guides/metrics/). Dogstatsd-rs supports most common DogStatsD metrics, including counters, gauges, histograms, sets, and tags. You can also use dogstatsd-rs to send custom events and time blocks of code. However, the client does not support service checks.

First, let's set up a counter to track the number of web page views.

We'll add these lines to the `hello` function in our `main.rs` before the declaration of
the `result` variable:

```
    // Binds to 127.0.0.1:8000 for transmitting and sends to
    // 127.0.0.1:8125, the default dogstatsd address
    let custom_options = Options::new("127.0.0.1:8000", "127.0.0.1:8125", "analytics");
    let custom_client = Client::new(custom_options);
    // Create a tag incrementing web page views
    custom_client.incr("web.page_views", vec!["tag:web.page_views".into()])
        .unwrap_or_else(|e| println!("Encountered error: {}", e));
```

Next, we'll create a histogram, which will record some statistics related to the time it takes to query our SQLite database.

In order to time the query, we need to import the [Instant struct](https://doc.rust-lang.org/std/time/struct.Instant.html) from the Rust standard library. This will enable us to get the current time in milliseconds.

Below our other import statements in `main.rs`, place the following:

```
use std::time::{Instant};
```

Since this import is from the standard library rather than an external one, we don't need to add anything to our `Cargo.toml`.

Now, after our counter code but before the declaration of `result` in our `hello` function, we'll add a line to get the current time:

```
    let start_time = Instant::now();
```

Below our declaration of `result`, which queries the database, we'll store our end time, calculate the difference between the start and end time in milliseconds, and feed the duration into Dogstatsd-rs's histogram function:

```
    let end_time = Instant::now();
    let duration = end_time.duration_since(start_time).as_secs();
    custom_client.histogram("database.query.time", &duration.to_string(), vec!["tag:database.query.time".into()])
      .unwrap_or_else(|e| println!("Encountered error: {}", e));
```

Finally, let's reload our web page several times so that we have more information to visualize in our graphs in the next section. Reloading the page will increment our counter while querying our database at the same time. Each successive request should yield this output in your terminal:

```
GET /:
    => Matched: GET /
    => Outcome: Success
    => Response succeeded.
```

## Part 3: Visualizing Server Metrics with Datadog

Now, let's head over to the Datadog Metric Explorer to visualize the metrics we've created.
As we expected, there's a sudden uptick at around 19:52 when we reloaded the
page 16 times, but the rest of the graph is flat.

![Web Page Views](/screenshots/Web Page Views.png?raw=true "Web Page Views")

What about our database queries? As described in the [DogStatsD docs](http://docs.datadoghq.com/guides/metrics/), creating a single histogram gives us 5 graphs to
visualize in the Metrics Explorer. Some of these graphs contain minimal information given the small size of our database, but it's useful to run through them nonetheless as a proof of concept.

First, `analytics.database.query.time.count` tells us the number of times the
metric was sampled - 0.53 on average in our case.

![Database Query Time Count](/screenshots/Database Query Time Count.png?raw=true "Database Query Time Count")

Second, `analytics.database.query.time.avg` tells us the average time of the sampled values - 0
in our case.

 ![Database Query Time Average](/screenshots/Database Query Time Avg.png?raw=true "Database Query Time Average")

Third, `analytics.database.query.time.median` tells us the median sampled value, which is also 0.

 ![Database Query Time Median](/screenshots/Database Query Time Median.png?raw=true "Database Query Time Median")

Fourth, `analytics.database.query.time.max` tells us the maximum sampled value - also 0.

![Database Query Time Max](/screenshots/Database Query Time Max.png?raw=true "Database Query Time Maximum")

Finally, `analytics.database.query.time.95percentile` tells us the 95th percentile sampled value, which is also 0.

![Database Query Time 95th Percentile](/screenshots/Database Query Time 95th Percentile.png?raw=true "Database Query Time 95th Percentile")

Again, for a larger database, you'll see more variation in your results.

## Conclusion

This concludes our tutorial. Next time you write a Rust application, consider integrating
dogstatsd-rs!

The Markdown and code for this post, which integrates the gists and snippets found above, is [on GitHub](https://github.com/outkaj/rocket-dogstatsd-example).
