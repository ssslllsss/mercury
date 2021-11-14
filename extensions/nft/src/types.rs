use common::PaginationRequest;
use core_rpc::types::Item;

use ckb_types::bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct QueryMnftNftTransactionsPayload {
    pub nft_address: Option<String>,
    pub nft_id: Option<NftIdentify>,
    pub pagination: PaginationRequest,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]

pub struct NftIdentify {
    pub issuer_id: String,
    pub class_id: u32,
    pub token_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]

pub struct NftIdentity {
    pub issuer_id: String,
    pub class_id: u32,
    pub token_id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct MnftIssuerCreatePayload {
    pub info: Bytes,
    pub issuer_address: String,
    pub from: Option<Item>,
    pub change: Option<String>,
    pub fee_rate: Option<u64>,
}
