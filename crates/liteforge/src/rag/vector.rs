//! Vector math utilities for similarity search.

/// Compute the dot product of two vectors.
///
/// # Example
/// ```
/// use liteforge::rag::dot_product;
///
/// let a = vec![1.0, 2.0, 3.0];
/// let b = vec![4.0, 5.0, 6.0];
/// assert_eq!(dot_product(&a, &b), 32.0); // 1*4 + 2*5 + 3*6
/// ```
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compute the magnitude (L2 norm) of a vector.
pub fn magnitude(v: &[f32]) -> f32 {
    dot_product(v, v).sqrt()
}

/// Normalize a vector to unit length.
///
/// Returns a zero vector if the input has zero magnitude.
///
/// # Example
/// ```
/// use liteforge::rag::normalize;
///
/// let v = vec![3.0, 4.0];
/// let n = normalize(&v);
/// assert!((n[0] - 0.6).abs() < 0.001);
/// assert!((n[1] - 0.8).abs() < 0.001);
/// ```
pub fn normalize(v: &[f32]) -> Vec<f32> {
    let mag = magnitude(v);
    if mag == 0.0 {
        vec![0.0; v.len()]
    } else {
        v.iter().map(|x| x / mag).collect()
    }
}

/// Compute cosine similarity between two vectors.
///
/// Returns a value between -1.0 and 1.0, where:
/// - 1.0 means vectors point in the same direction
/// - 0.0 means vectors are orthogonal
/// - -1.0 means vectors point in opposite directions
///
/// # Example
/// ```
/// use liteforge::rag::cosine_similarity;
///
/// let a = vec![1.0, 0.0];
/// let b = vec![1.0, 0.0];
/// assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
///
/// let c = vec![0.0, 1.0];
/// assert!(cosine_similarity(&a, &c).abs() < 0.001); // orthogonal
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let mag_a = magnitude(a);
    let mag_b = magnitude(b);

    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

/// Compute Euclidean distance between two vectors.
///
/// # Example
/// ```
/// use liteforge::rag::euclidean_distance;
///
/// let a = vec![0.0, 0.0];
/// let b = vec![3.0, 4.0];
/// assert!((euclidean_distance(&a, &b) - 5.0).abs() < 0.001);
/// ```
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_product() {
        assert_eq!(dot_product(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]), 32.0);
        assert_eq!(dot_product(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
    }

    #[test]
    fn test_magnitude() {
        assert!((magnitude(&[3.0, 4.0]) - 5.0).abs() < 0.001);
        assert_eq!(magnitude(&[0.0, 0.0]), 0.0);
    }

    #[test]
    fn test_normalize() {
        let n = normalize(&[3.0, 4.0]);
        assert!((n[0] - 0.6).abs() < 0.001);
        assert!((n[1] - 0.8).abs() < 0.001);
        assert!((magnitude(&n) - 1.0).abs() < 0.001);

        // Zero vector
        let z = normalize(&[0.0, 0.0]);
        assert_eq!(z, vec![0.0, 0.0]);
    }

    #[test]
    fn test_cosine_similarity() {
        // Same direction
        assert!((cosine_similarity(&[1.0, 0.0], &[2.0, 0.0]) - 1.0).abs() < 0.001);

        // Opposite direction
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 0.001);

        // Orthogonal
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 0.001);

        // 45 degrees
        let sim = cosine_similarity(&[1.0, 0.0], &[1.0, 1.0]);
        assert!((sim - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_euclidean_distance() {
        assert!((euclidean_distance(&[0.0, 0.0], &[3.0, 4.0]) - 5.0).abs() < 0.001);
        assert_eq!(euclidean_distance(&[1.0, 2.0], &[1.0, 2.0]), 0.0);
    }
}
