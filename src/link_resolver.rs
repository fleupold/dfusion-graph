use graph::components::link_resolver::JsonValueStream;
use graph::data::subgraph::Link;
use graph::prelude::LinkResolver;

use futures::prelude::*;
use futures::future::*;
use slog::Logger;
use std::fs::File;
use std::io::prelude::*;

pub struct DummyLinkResolver {}

impl LinkResolver for DummyLinkResolver {
    /// Fetches the link contents as bytes.
    fn cat(
        &self,
        _logger: &Logger,
        link: &Link,
    ) -> Box<Future<Item = Vec<u8>, Error = failure::Error> + Send> {
        let mut f = if link.link == "/ipfs/deploymentid" {
            File::open("subgraph.yaml").unwrap()
        } else {
            File::open(&link.link).unwrap()
        };
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap();
        Box::new(ok(buffer))
    }

    fn json_stream(
        &self,
        _link: &Link,
    ) -> Box<Future<Item = JsonValueStream, Error = failure::Error> + Send + 'static> {
        unimplemented!();
    }
}
