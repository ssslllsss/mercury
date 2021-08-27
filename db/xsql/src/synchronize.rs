use crate::table::{
    BlockTable, CanonicalChainTable, CellTable, LiveCellTable, ScriptTable, TransactionTable,
    UncleRelationshipTable,
};
use crate::{generate_id, to_bson_bytes, DBAdapter};

use common::anyhow::Result;

use ckb_types::{core::BlockNumber, packed, prelude::*};
use futures::stream::StreamExt;
use rbatis::crud::{CRUDMut, CRUD};
use rbatis::{rbatis::Rbatis, wrapper::Wrapper};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::collections::HashSet;
use std::sync::Arc;

const MAX_OUT_POINT_QUEUE_SIZE: usize = 5000;

macro_rules! save_list {
	($tx: expr$ (, $table_list: expr)*) => {{
		$($tx.save_batch(&$table_list, &[]).await?;)*
		$tx.commit().await?;
	}};
}

pub async fn sync_blocks_process<T: DBAdapter>(
    rb: Arc<Rbatis>,
    adapter: Arc<dyn DBAdapter>,
    block_list: Vec<BlockNumber>,
    outpoint_tx: UnboundedSender<packed::OutPoint>,
    number_tx: UnboundedSender<u64>,
    batch_size: usize,
) -> Result<()> {
    let mut max_number = BlockNumber::MIN;

    for numbers in block_list.chunks(batch_size).into_iter() {
        let mut tx = rb.acquire_begin().await?;
        let blocks = adapter.pull_blocks(numbers.to_vec()).await?;
        let mut block_table_batch: Vec<BlockTable> = Vec::new();
        let mut tx_table_batch: Vec<TransactionTable> = Vec::new();
        let mut cell_table_batch: Vec<CellTable> = Vec::new();
        let mut script_table_batch: HashSet<ScriptTable> = HashSet::new();
        let mut uncle_relationship_table_batch: Vec<UncleRelationshipTable> = Vec::new();
        let mut canonical_data_table_batch: Vec<CanonicalChainTable> = Vec::new();

        for block in blocks.iter() {
            let block_number = block.number();
            let block_hash = block.hash().raw_data().to_vec();
            let block_timestamp = block.timestamp();
            let block_epoch = block.epoch();
            max_number = max_number.max(block_number);

            block_table_batch.push(block.into());
            uncle_relationship_table_batch.push(UncleRelationshipTable {
                block_hash: to_bson_bytes(&block_hash),
                uncle_hashes: to_bson_bytes(&block.uncle_hashes().as_bytes()),
            });
            canonical_data_table_batch.push(CanonicalChainTable {
                block_number,
                block_hash: to_bson_bytes(&block_hash),
            });

            for (idx, tx) in block.transactions().iter().enumerate() {
                let tx_hash = to_bson_bytes(&tx.hash().raw_data());
                tx_table_batch.push(TransactionTable::from_view(
                    tx,
                    generate_id(block_number),
                    idx as u16,
                    to_bson_bytes(&block_hash),
                    block_number,
                    block_timestamp,
                ));

                for (i, (cell, data)) in tx.outputs_with_data_iter().enumerate() {
                    let cell_table = CellTable::from_cell(
                        &cell,
                        generate_id(block_number),
                        tx_hash.clone(),
                        i as u16,
                        idx as u16,
                        block_number,
                        to_bson_bytes(&block_hash),
                        block_epoch,
                        &data,
                    );

                    script_table_batch
                        .insert(cell_table.to_lock_script_table(generate_id(block_number)));

                    if cell_table.has_type_script() {
                        script_table_batch
                            .insert(cell_table.to_type_script_table(generate_id(block_number)));
                    }

                    cell_table_batch.push(cell_table);
                }

                tx.inputs()
                    .into_iter()
                    .for_each(|input| outpoint_tx.send(input.previous_output()).unwrap());
            }
        }

        let live_cell_table_batch = cell_table_batch
            .clone()
            .into_iter()
            .map(Into::into)
            .collect::<Vec<LiveCellTable>>();
        let script_table_batch = script_table_batch.into_iter().collect::<Vec<_>>();

        save_list!(
            tx,
            block_table_batch,
            tx_table_batch,
            cell_table_batch,
            live_cell_table_batch,
            script_table_batch,
            uncle_relationship_table_batch,
            canonical_data_table_batch
        );
    }

    number_tx.send(max_number).unwrap();

    Ok(())
}

pub async fn handle_out_point(
    rb: Arc<Rbatis>,
    rx: UnboundedReceiver<packed::OutPoint>,
) -> Result<()> {
    let mut queue = Vec::new();
    let mut stream = UnboundedReceiverStream::new(rx);
    let mut threshold = MAX_OUT_POINT_QUEUE_SIZE;
    let wrapper = rb.new_wrapper_table::<LiveCellTable>();

    while let Some(out_point) = stream.next().await {
        let tx_hash = to_bson_bytes(&out_point.tx_hash().raw_data());
        let index: u32 = out_point.index().unpack();
        let w = build_wrapper(&wrapper, &tx_hash.bytes, index);

        if !try_remove_live_cell(Arc::clone(&rb), &w).await? {
            queue.push(InnerOutPoint::new(tx_hash.bytes, index));
        }

        if queue.len() >= threshold {
            while let Some(item) = queue.pop() {
                let w = build_wrapper(&wrapper, &item.tx_hash, item.index);
                if !try_remove_live_cell(Arc::clone(&rb), &w).await? {
                    queue.push(item);
                }
            }

            threshold += 1000;
        }
    }

    Ok(())
}

fn build_wrapper(wrapper: &Wrapper, tx_hash: &[u8], output_index: u32) -> Wrapper {
    let w = wrapper.clone();
    w.eq("tx_hash", tx_hash)
        .and()
        .eq("output_index", output_index)
}

async fn try_remove_live_cell(rb: Arc<Rbatis>, wrapper: &Wrapper) -> Result<bool> {
    let ra = rb.remove_by_wrapper::<LiveCellTable>(wrapper).await?;
    Ok(ra == 1)
}

struct InnerOutPoint {
    tx_hash: Vec<u8>,
    index: u32,
}

impl InnerOutPoint {
    fn new(tx_hash: Vec<u8>, index: u32) -> Self {
        InnerOutPoint { tx_hash, index }
    }
}