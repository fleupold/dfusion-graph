use graph::components::EventProducer;
use graph::components::link_resolver::JsonValueStream;
use graph::components::subgraph::SubgraphAssignmentProvider;
use graph::data::schema::Schema;
use graph::data::subgraph::{
    BaseSubgraphManifest, SubgraphDeploymentId, Source, Link, DataSource,
    Mapping, MappingEventHandler, SubgraphAssignmentProviderEvent,
    SubgraphAssignmentProviderError
};
use graph::prelude::LinkResolver;

use graph_store_postgres::Store as DieselStore;

use graphql_parser::schema::Document;

use futures::prelude::*;
use futures::future::*;
use parity_wasm::elements::Module;
use slog::Logger;
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::sync::Arc;
use web3::types::{Address};

pub struct DummySubgraphProvider {
    already_produced: bool,
    logger: Logger,
    store: Arc<DieselStore>,
    manifest: BaseSubgraphManifest::<Schema, DataSource>,
}

impl DummySubgraphProvider {
    pub fn new(logger: Logger, store: Arc<DieselStore>) -> DummySubgraphProvider {
        let address = Address::from_str("51E06e232EB9959B90ba788c4e4d61ffD528520C").unwrap();
        let data_sources = vec![
            DataSource {
                kind: "DataSource".to_owned(),
                network: None,
                name: "getCurrentStateRoot".to_owned(),
                source: Source {
                    address: Some(address),
                    abi: "1234".to_owned(),
                },
                mapping: Mapping {
                    kind: "Mapping".to_owned(),
                    api_version: "1".to_owned(),
                    language: "language".to_owned(),
                    entities: vec![],
                    abis: vec![],
                    block_handlers: None,
                    call_handlers: None,
                    event_handlers: Some(vec![
                        MappingEventHandler {
                            event: "Foo()".to_owned(),
                            topic0: None,
                            handler: "".to_owned(),
                        }
                    ]),
                    runtime: Arc::new(Module::default()),
                    link: Link::from("Link".to_owned()),
                },
                templates: None
            }
        ];
        let schema = Schema::new(
            SubgraphDeploymentId::new("testschema").unwrap(),
            Document {definitions: vec![]},
        );
        let manifest = BaseSubgraphManifest::<Schema, DataSource> {
            id: SubgraphDeploymentId::new("testmanifest").unwrap(),
            location: "test_location".to_owned(),
            spec_version: "test_spec_version".to_owned(),
            description: None,
            repository: None,
            schema,
            data_sources: data_sources,
        };

        DummySubgraphProvider {
            already_produced: false,
            logger,
            store,
            manifest,
        }
    }
}

impl EventProducer<SubgraphAssignmentProviderEvent> for DummySubgraphProvider {
    fn take_event_stream(&mut self) -> Option<Box<Stream<Item = SubgraphAssignmentProviderEvent, Error = ()> + Send>> {
        Some(Box::new(DummySubgraphProvider::new(self.logger.clone(), self.store.clone())))
    }
}

impl Stream for DummySubgraphProvider {
    type Item = SubgraphAssignmentProviderEvent;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self.already_produced {
            Ok(Async::Ready(None))
        } else {
            self.already_produced = true;
            let event = SubgraphAssignmentProviderEvent::SubgraphStart(self.manifest.clone());
            Ok(Async::Ready(Some(event)))
        }
    }
}

impl LinkResolver for DummySubgraphProvider {
    /// Fetches the link contents as bytes.
    fn cat(
        &self,
        _logger: &Logger,
        link: &Link,
    ) -> Box<Future<Item = Vec<u8>, Error = failure::Error> + Send> {
        let mut f = if link.link == "schema.graphql" {
            File::open("schema.graphql").unwrap()
        } else {
            File::open("subgraph.yaml").unwrap()
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

impl SubgraphAssignmentProvider for DummySubgraphProvider {
    fn start(
        &self,
        _id: SubgraphDeploymentId,
    ) -> Box<Future<Item = (), Error = SubgraphAssignmentProviderError> + Send + 'static> {
        unimplemented!();
    }

    fn stop(
        &self,
        _id: SubgraphDeploymentId,
    ) -> Box<Future<Item = (), Error = SubgraphAssignmentProviderError> + Send + 'static> {
        unimplemented!();
    }
}
