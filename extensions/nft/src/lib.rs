mod types;

use crate::types::{MnftIssuerCreatePayload, QueryMnftNftTransactionsPayload};

use common::{
    async_trait, utils::ScriptInfo, Context, PaginationResponse, Result, TransactionWrapper,
};
use core_rpc::rpc_impl::utils::RpcUtility;
use core_rpc::types::TransactionCompletionResponse;
use core_storage::ExtensionStorage;

use ckb_jsonrpc_types::TransactionWithStatus;
use ckb_types::{bytes::Bytes, packed, prelude::*, H256};
use jsonrpsee_http_server::types::Error;
use jsonrpsee_proc_macros::rpc;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[rpc(server)]
pub trait NFTRpc {
    #[method(name = "build_mnft_issuer_creation_transaction")]
    async fn build_mnft_issuer_creation_transaction(
        &self,
        payload: MnftIssuerCreatePayload,
    ) -> Result<TransactionCompletionResponse, Error>;

    #[method(name = "query_mnft_nft_transactions")]
    async fn query_mnft_nft_transactions(
        &self,
        payload: QueryMnftNftTransactionsPayload,
    ) -> Result<Vec<TransactionWithStatus>, Error>;
}

#[derive(Clone)]
pub struct NftExtension<SE, RU> {
    _store: SE,
    _utils: RU,
    _builtin_scripts: HashMap<String, ScriptInfo>,
}

#[async_trait]
impl<SE, RU> NFTRpcServer for NftExtension<SE, RU>
where
    SE: ExtensionStorage + Sync + Send + 'static,
    RU: RpcUtility + Send + Sync + 'static,
{
    async fn build_mnft_issuer_creation_transaction(
        &self,
        payload: MnftIssuerCreatePayload,
    ) -> Result<TransactionCompletionResponse, Error> {
        let mut outputs = Vec::new();

        let script_builder = self
            ._builtin_scripts
            .get("nft")
            .cloned()
            .unwrap()
            .script
            .as_builder();
        let cell = packed::CellOutputBuilder::default()
            .type_(Some(script_builder.args(type_id()).build()).pack())
            .capacity(131u64.pack())
            .build();
        outputs.push(cell);

        let mut inputs = Vec::new();
        let mut scripts = HashSet::new();
        let mut sig_action = HashMap::new();
        let mut input_index = 0;
        self._utils
            .pool_live_cells_by_items(
                Context::new(),
                vec![payload.from.clone().unwrap()],
                131 * 10000000,
                vec![],
                None,
                &mut 0,
                &mut inputs,
                &mut scripts,
                &mut sig_action,
                &mut input_index,
            )
            .await?;

        let (view, entry) = self._utils.prebuild_tx_complete(
            vec![],
            outputs,
            vec![],
            scripts,
            vec![],
            sig_action,
            HashMap::new(),
        )?;

        Ok(TransactionCompletionResponse {
            tx_view: view,
            signature_actions: entry,
        })
    }

    async fn query_mnft_nft_transactions(
        &self,
        payload: QueryMnftNftTransactionsPayload,
    ) -> Result<Vec<TransactionWithStatus>, Error> {
        let mut type_scripts = Vec::new();
        if let Some(addr) = payload.nft_address.clone() {
            type_scripts.push(H256::from_str(&addr).unwrap());
        }

        if let Some(id) = payload.nft_id.clone() {
            let mut args = Vec::new();
            args.extend_from_slice(id.issuer_id.as_bytes());
            args.extend_from_slice(&id.class_id.to_le_bytes());
            args.extend_from_slice(&id.token_id.to_le_bytes());
            let script_builder = self
                ._builtin_scripts
                .get("nft")
                .cloned()
                .unwrap()
                .script
                .as_builder();
            type_scripts.push(
                script_builder
                    .args(Bytes::from(args).pack())
                    .build()
                    .calc_script_hash()
                    .unpack(),
            );
        }

        let txs = self
            ._store
            .get_transactions_by_scripts(Context::new(), vec![], type_scripts, None)
            .await?;

        Ok(txs
            .into_iter()
            .map(|tx| tx.transaction_with_status)
            .collect())
    }
}

impl<SE, RU> NftExtension<SE, RU>
where
    SE: ExtensionStorage + Sync + Send + 'static,
    RU: RpcUtility + Send + Sync + 'static,
{
    pub fn new(
        _store: SE,
        _utils: RU,
        _builtin_scripts: HashMap<String, ScriptInfo>,
        _extra_config: Bytes,
    ) -> Self {
        NftExtension {
            _store,
            _utils,
            _builtin_scripts,
        }
    }
}

fn type_id() -> packed::Bytes {
    Default::default()
}
