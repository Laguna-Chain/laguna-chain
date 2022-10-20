use fc_rpc_core::{types::PeerCount, NetApiServer};

use jsonrpsee::core::RpcResult as Result;
use pallet_evm_compat_rpc::EvmCompatApiRuntimeApi as EvmCompatRuntimeApi;
use primitives::{AccountId, Balance};
use sc_client_api::HeaderBackend;
use sc_network::{ExHashT, NetworkService};

use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

use super::internal_err;
use std::sync::Arc;

pub struct Net<B: BlockT, C, H: ExHashT> {
	client: Arc<C>,
	network: Arc<NetworkService<B, H>>,
	peer_count_as_hex: bool,
}

impl<B: BlockT, C, H: ExHashT> Net<B, C, H> {
	pub fn new(
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
		peer_count_as_hex: bool,
	) -> Self {
		Self { client, network, peer_count_as_hex }
	}
}

impl<B, C, H: ExHashT> NetApiServer for Net<B, C, H>
where
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C: HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync + 'static,
	C::Api: EvmCompatRuntimeApi<B, AccountId, Balance>,
{
	fn version(&self) -> Result<String> {
		let hash = self.client.info().best_hash;
		Ok(self
			.client
			.runtime_api()
			.chain_id(&BlockId::Hash(hash))
			.map_err(|_| internal_err("fetch runtime version failed"))?
			.to_string())
	}

	fn peer_count(&self) -> Result<PeerCount> {
		let peer_count = self.network.num_connected();
		Ok(match self.peer_count_as_hex {
			true => PeerCount::String(format!("0x{:x}", peer_count)),
			false => PeerCount::U32(peer_count as u32),
		})
	}

	fn is_listening(&self) -> Result<bool> {
		Ok(true)
	}
}
