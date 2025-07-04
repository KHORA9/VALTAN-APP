use sqlx::SqlitePool;
use crate::CodexResult;

pub struct VectorOps;

impl VectorOps {
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if mag_a == 0.0 || mag_b == 0.0 {
            0.0
        } else {
            dot / (mag_a * mag_b)
        }
    }
    
    /// Store vector as binary BLOB for efficiency
    pub async fn store_vector(
        pool: &SqlitePool,
        doc_id: &str,
        vector: &[f32],
        model: &str,
    ) -> CodexResult<()> {
        let blob = bincode::serialize(vector)?;
        
        sqlx::query(
            "INSERT INTO embeddings (id, document_id, vector_blob, dimensions, model, chunk_index, text_chunk, start_position, end_position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, '', 0, 0, datetime('now'))"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(doc_id)
        .bind(blob)
        .bind(vector.len() as i64)
        .bind(model)
        .execute(pool)
        .await?;
        
        Ok(())
    }
}