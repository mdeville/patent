//! Semantic ranking (M3).
//!
//! Embeds the idea and each match description with `fastembed`, computes cosine
//! similarity, dedups, sorts, and keeps the top N.

use crate::model::{Match, Query};

/// Default number of matches to keep after ranking.
pub const DEFAULT_LIMIT: usize = 15;

/// Cosine similarity between two equal-length vectors, in `[-1.0, 1.0]`.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

/// Embed the idea + descriptions, rank by cosine similarity, keep top `limit`.
pub fn rank(_query: &Query, _matches: Vec<Match>, _limit: usize) -> crate::Result<Vec<Match>> {
    todo!("M3: fastembed embed + cosine rank + top-N")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_is_one() {
        let v = [1.0, 2.0, 3.0];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        assert!((cosine(&[1.0, 0.0], &[0.0, 1.0])).abs() < 1e-6);
    }

    #[test]
    fn cosine_zero_vector_is_zero() {
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }
}
