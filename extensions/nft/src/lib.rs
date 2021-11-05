use common::{async_trait, utils::ScriptInfo, Result};
use core_storage::Storage;

use ckb_types::bytes::Bytes;
use jsonrpsee_http_server::types::Error;
use jsonrpsee_proc_macros::rpc;

use std::collections::HashMap;

#[rpc(server)]
pub trait NFTRpc {
    #[method(name = "nft")]
    async fn nft(&self) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct NftExtension<S> {
    _store: S,
    _builtin_scripts: HashMap<String, ScriptInfo>,
}

#[async_trait]
impl<S: Storage + Sync + Send + 'static> NFTRpcServer for NftExtension<S> {
    async fn nft(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl<S: Storage + Sync + Send + 'static> NftExtension<S> {
    pub fn new(
        _store: S,
        _builtin_scripts: HashMap<String, ScriptInfo>,
        _extra_config: Bytes,
    ) -> Self {
        NftExtension {
            _store,
            _builtin_scripts,
        }
    }
}
