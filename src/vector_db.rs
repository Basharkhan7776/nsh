use anyhow::Result;
use arrow::array::{FixedSizeListArray, RecordBatch, StringArray, Float32Array, ArrayRef};
use arrow::datatypes::{DataType, Field, Schema, Float32Type};
use arrow::record_batch::RecordBatchIterator;
use lancedb::{connect, connection::Connection};
use futures::TryStreamExt;
use std::sync::Arc;
use uuid::Uuid;

pub struct VectorStore {
    conn: Connection,
}

impl VectorStore {
    pub async fn new(path: &str) -> Result<Self> {
        let conn = connect(path).execute().await?;
        Ok(Self { conn })
    }

    pub async fn add_texts(&self, texts: Vec<String>, embeddings: Vec<Vec<f32>>) -> Result<()> {
        if texts.is_empty() {
            return Ok(());
        }

        let dim = embeddings[0].len();
        let total_rows = texts.len();

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("text", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dim as i32,
                ),
                false,
            ),
        ]));

        let mut ids = Vec::with_capacity(total_rows);
        let mut flat_vectors = Vec::with_capacity(total_rows * dim);

        for _ in 0..total_rows {
            ids.push(Uuid::new_v4().to_string());
        }

        for vec in &embeddings {
            flat_vectors.extend_from_slice(vec);
        }

        let id_array = StringArray::from(ids);
        let text_array = StringArray::from(texts);
        
        let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            embeddings.into_iter().map(|v| Some(v.into_iter().map(Some))),
            dim as i32
        );

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array) as ArrayRef,
                Arc::new(text_array) as ArrayRef,
                Arc::new(vector_array) as ArrayRef,
            ],
        )?;

        let table_name = "vectors";
        let batches = vec![batch];
        
        // LanceDB expects Box<dyn RecordBatchReader>
        let reader = Box::new(RecordBatchIterator::new(
            batches.into_iter().map(Ok),
            schema.clone()
        ));

        if self.conn.table_names().execute().await?.contains(&table_name.to_string()) {
            let tbl = self.conn.open_table(table_name).execute().await?;
            tbl.add(reader).execute().await?;
        } else {
            self.conn.create_table(table_name, reader).execute().await?;
        };

        Ok(())
    }

    pub async fn search(&self, query_vector: Vec<f32>, limit: usize) -> Result<Vec<(String, f32)>> {
        let table_name = "vectors";
        if !self.conn.table_names().execute().await?.contains(&table_name.to_string()) {
            return Ok(vec![]);
        }

        let table = self.conn.open_table(table_name).execute().await?;
        
        let mut results = table
            .query()
            .nearest_to(&query_vector)
            .limit(limit)
            .execute_stream()
            .await?;

        let mut output = Vec::new();
        
        while let Some(batch) = results.try_next().await? {
             let text_col = batch.column_by_name("text").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
             let dist_col_opt = batch.column_by_name("_distance");
             
             for i in 0..batch.num_rows() {
                 let text = text_col.value(i).to_string();
                 let dist = if let Some(d) = dist_col_opt {
                     d.as_any().downcast_ref::<Float32Array>().unwrap().value(i)
                 } else {
                     0.0
                 };
                 output.push((text, dist));
             }
        }

        Ok(output)
    }
}
