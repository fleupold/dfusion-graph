extern crate env_logger;
extern crate graph;
extern crate graph_core;
extern crate graph_datasource_ethereum;
extern crate lazy_static;

mod runtime_host;
mod subgraph_provider;

use lazy_static::lazy_static;
use std::sync::Arc;
use std::time::Duration;

use graph::components::forward;
use graph::prelude::{SubgraphRegistrar as SubgraphRegistrarTrait, *};
use graph::log::logger;
use graph::tokio_executor;
use graph::tokio_timer;
use graph::tokio_timer::timer::Timer;

use graph_core::{SubgraphInstanceManager, SubgraphRegistrar};

use graph_datasource_ethereum::{BlockStreamBuilder, Transport};

use graph_store_postgres::{Store as DieselStore, StoreConfig};

use runtime_host::DummyRuntimeHost;
use subgraph_provider::DummySubgraphProvider;

lazy_static! {
    static ref ANCESTOR_COUNT: u64 = 50;
    static ref REORG_THRESHOLD: u64 = 50;
}

fn main() {
    use std::sync::Mutex;
    use tokio::runtime;

    // Create components for tokio context: multi-threaded runtime, executor
    // context on the runtime, and Timer handle.
    //
    // Configure the runtime to shutdown after a panic.
    let runtime: Arc<Mutex<Option<runtime::Runtime>>> = Arc::new(Mutex::new(None));
    let handler_runtime = runtime.clone();
    *runtime.lock().unwrap() = Some(
        runtime::Builder::new()
            .core_threads(100)
            .panic_handler(move |_| {
                let runtime = handler_runtime.clone();
                std::thread::spawn(move || {
                    if let Some(runtime) = runtime.lock().unwrap().take() {
                        // Try to cleanly shutdown the runtime, but
                        // unconditionally exit after a while.
                        std::thread::spawn(|| {
                            std::thread::sleep(Duration::from_millis(3000));
                            std::process::exit(1);
                        });
                        runtime
                            .shutdown_now()
                            .wait()
                            .expect("Failed to shutdown Tokio Runtime");
                        println!("Runtime cleaned up and shutdown successfully");
                    }
                });
            })
            .build()
            .unwrap(),
    );

    let mut executor = runtime.lock().unwrap().as_ref().unwrap().executor();
    let mut enter = tokio_executor::enter()
        .expect("Failed to enter runtime executor, multiple executors at once");
    let timer = Timer::default();
    let timer_handle = timer.handle();

    // Setup runtime context with defaults and run the main application
    tokio_executor::with_default(&mut executor, &mut enter, |enter| {
        tokio_timer::with_default(&timer_handle, enter, |enter| {
            enter
                .block_on(future::lazy(|| async_main()))
                .expect("Failed to run main function");
        })
    });
}

fn async_main() -> impl Future<Item = (), Error = ()> + Send + 'static {
    env_logger::init();

    let ethereum_network_name : &str = "dev3";
    let block_polling_interval = Duration::from_millis(1000);
    let node_id = NodeId::new("test_node").unwrap();

    let logger = logger(false);
    let logger_factory = LoggerFactory::new(logger.clone(), None);

    // Set up Ethereum transport
    let (transport_event_loop, transport) = Transport::new_rpc("http://localhost:8545");

    // If we drop the event loop the transport will stop working.
    // For now it's fine to just leak it.
    std::mem::forget(transport_event_loop);

    let eth_adapter = Arc::new(graph_datasource_ethereum::EthereumAdapter::new(
        transport,
        0,
    ));
    let eth_net_identifiers = match eth_adapter.net_identifiers(&logger).wait() {
        Ok(net) => {
            info!(
                logger, "Connected to Ethereum";
            );
            net
        }
        Err(e) => {
            error!(logger, "Was a valid Ethereum node provided?");
            panic!("Failed to connect to Ethereum node: {}", e);
        }
    };

    let store = Arc::new(DieselStore::new(
        StoreConfig {
            postgres_url: "postgresql://graph-node:let-me-in@localhost/graph-node".to_owned(),
            network_name: ethereum_network_name.to_owned(),
            start_block: 0,
        },
        &logger,
        eth_net_identifiers,
    ));

    // Create Ethereum block ingestor
    let block_ingestor = graph_datasource_ethereum::BlockIngestor::new(
        store.clone(),
        eth_adapter.clone(),
        *ANCESTOR_COUNT,
        ethereum_network_name.to_string(),
        &logger_factory,
        block_polling_interval,
    )
    .expect("failed to create Ethereum block ingestor");

    // Run the Ethereum block ingestor in the background
    tokio::spawn(block_ingestor.into_polling_stream());

    // Prepare a block stream builder for subgraphs
    let block_stream_builder = BlockStreamBuilder::new(
        store.clone(),
        store.clone(),
        eth_adapter.clone(),
        node_id.clone(),
        *REORG_THRESHOLD,
    );

    let subgraph_instance_manager = SubgraphInstanceManager::new(
        &logger_factory,
        store.clone(),
        DummyRuntimeHost {},
        block_stream_builder,
    );

    let mut subgraph_provider = DummySubgraphProvider::new(logger.clone(), store.clone());

    // Forward subgraph events from the subgraph provider to the subgraph instance manager
    tokio::spawn(forward(&mut subgraph_provider, &subgraph_instance_manager).unwrap());

    let subgraph_arc = Arc::new(subgraph_provider);

    // Create named subgraph provider for resolving subgraph name->ID mappings	
    let subgraph_registrar = Arc::new(SubgraphRegistrar::new(	
        &logger_factory,	
        subgraph_arc.clone(),	
        subgraph_arc.clone(),	
        store.clone(),	
        store.clone(),	
        node_id.clone(),	
        SubgraphVersionSwitchingMode::Instant
    ));
    tokio::spawn(	
        subgraph_registrar	
            .start()	
            .then(|start_result| Ok(start_result.expect("failed to initialize subgraph provider"))),	
    );

    let name = SubgraphName::new("subgraph").unwrap();
    let subgraph_id = SubgraphDeploymentId::new("deploymentid").unwrap();
    let node_id = NodeId::new("nodeId").unwrap();

    tokio::spawn(	
        subgraph_registrar	
            .create_subgraph_version(name, subgraph_id, node_id)		
            .then(|result| {	
                Ok(result.expect("Failed to deploy subgraph from `--subgraph` flag"))	
            }),	
    );

    future::empty()
}
