use failure::Error;
use futures::future::*;
use slog::Logger;
use std::sync::Arc;

use graph::components::subgraph::{RuntimeHost, RuntimeHostBuilder, BlockState};
use graph::components::ethereum::{EthereumCall, EthereumBlockTriggerType, EthereumBlock}
;
use graph::data::subgraph::{DataSource, SubgraphDeploymentId};

use tiny_keccak::keccak256;
use web3::types::{Log, Transaction};

#[derive(Clone, Debug)]
pub struct DummyRuntimeHost {}

impl RuntimeHostBuilder for DummyRuntimeHost {
    type Host = DummyRuntimeHost;
    fn build(
        &self,
        _logger: &Logger,
        _subgraph_id: SubgraphDeploymentId,
        _data_source: DataSource,
    ) -> Result<Self::Host, Error> {
        Ok(DummyRuntimeHost {})
    }
}

impl RuntimeHost for DummyRuntimeHost {
    fn matches_log(&self, _log: &Log) -> bool {
        true
    }

    fn matches_call(&self, call: &EthereumCall) -> bool {
        let target_method_id = &call.input.0[..4];
        println!("Matching call {:?}", target_method_id);
        let fhash = keccak256("getCurrentStateRoot()".as_bytes());
        let actual_method_id = [fhash[0], fhash[1], fhash[2], fhash[3]];
        target_method_id == actual_method_id
    }

    /// Returns true if the RuntimeHost has a handler for an Ethereum block.
    fn matches_block(&self, _call: EthereumBlockTriggerType) -> bool {
        println!("Matching Block");
        false
    }

    /// Process an Ethereum event and return a vector of entity operations.
    fn process_log(
        &self,
        _logger: Logger,
        _block: Arc<EthereumBlock>,
        _transaction: Arc<Transaction>,
        _log: Arc<Log>,
        state: BlockState,
    ) -> Box<Future<Item = BlockState, Error = Error> + Send> {
        println!("Received Log Event");
        Box::new(ok(state))
    }

    /// Process an Ethereum call and return a vector of entity operations
    fn process_call(
        &self,
        _logger: Logger,
        _block: Arc<EthereumBlock>,
        _transaction: Arc<Transaction>,
        _call: Arc<EthereumCall>,
        state: BlockState,
    ) -> Box<Future<Item = BlockState, Error = Error> + Send> {
        println!("Received Event");
        Box::new(ok(state))
    }

    /// Process an Ethereum block and return a vector of entity operations
    fn process_block(
        &self,
        _logger: Logger,
        _block: Arc<EthereumBlock>,
        _trigger_type: EthereumBlockTriggerType,
        state: BlockState,
    ) -> Box<Future<Item = BlockState, Error = Error> + Send> {
        Box::new(ok(state))
    }
}