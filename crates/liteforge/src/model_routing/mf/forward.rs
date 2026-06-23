//! Matrix-factorization forward pass.
//!
//! Mirrors RouteLLM's `MFModel`: project the prompt embedding (optionally),
//! take the Hadamard product with each L2-normalized anchor model row, run the
//! linear classifier, and convert the strong/weak logit difference to a scalar
//! "hardness" via the sigmoid. The result is the probability the strong model
//! is needed for this prompt, independent of our deployment groups.

use super::weights::MfWeights;
use crate::error::{ForgeError, Result};
use crate::rag::normalize;

/// Row-major matrix-vector product with bias: `out[j] = sum_i v[i]*m[i*cols+j] + bias[j]`.
///
/// `m` has length `rows*cols`, `v` has length `rows`, `bias` has length `cols`.
pub fn matvec(m: &[f32], rows: usize, cols: usize, v: &[f32], bias: &[f32]) -> Vec<f32> {
    debug_assert_eq!(v.len(), rows);
    debug_assert_eq!(m.len(), rows * cols);
    let mut out = bias.to_vec();
    for (i, &vi) in v.iter().enumerate() {
        if vi == 0.0 {
            continue;
        }
        let base = i * cols;
        let row = &m[base..base + cols];
        for (o, &w) in out.iter_mut().zip(row.iter()) {
            *o += vi * w;
        }
    }
    out
}

/// Numerically-stable logistic sigmoid.
pub fn sigmoid(x: f32) -> f32 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let e = x.exp();
        e / (1.0 + e)
    }
}

fn hadamard(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).collect()
}

/// Project the prompt embedding to the latent dimension `d` if configured.
fn project(weights: &MfWeights, e: &[f32]) -> Vec<f32> {
    if weights.use_proj {
        let proj_w = weights.proj_w.as_ref().expect("validated: proj_w present");
        let zeros;
        let bias = match &weights.proj_b {
            Some(b) => b.as_slice(),
            None => {
                zeros = vec![0.0f32; weights.d];
                &zeros
            }
        };
        matvec(proj_w, weights.text_dim, weights.d, e, bias)
    } else {
        e.to_vec()
    }
}

/// Classifier logits for one anchor row (already L2-normalized) and a projected
/// prompt vector.
fn anchor_logits(weights: &MfWeights, anchor_norm: &[f32], pe: &[f32]) -> Vec<f32> {
    let interaction = hadamard(anchor_norm, pe);
    matvec(
        &weights.cls_w,
        weights.d,
        weights.num_classes,
        &interaction,
        &weights.cls_b,
    )
}

/// Compute the scalar hardness in (0, 1) for a prompt embedding `e`.
///
/// `e` must have length `weights.text_dim`.
pub fn mf_hardness(weights: &MfWeights, e: &[f32]) -> Result<f32> {
    if e.len() != weights.text_dim {
        return Err(ForgeError::internal(format!(
            "MF hardness: embedding length {} != text_dim {}",
            e.len(),
            weights.text_dim
        )));
    }
    let pe = project(weights, e);
    let strong = normalize(&weights.strong_row);
    let weak = normalize(&weights.weak_row);
    let ls = anchor_logits(weights, &strong, &pe);
    let lw = anchor_logits(weights, &weak, &pe);
    let diff = ls[weights.strong_class] - lw[weights.weak_class];
    Ok(sigmoid(diff))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matvec_matches_hand_computation() {
        // m = [[1,2],[3,4],[5,6]] (rows=3, cols=2), v=[1,0,2], bias=[10,20]
        // out[0] = 1*1 + 0*3 + 2*5 + 10 = 21
        // out[1] = 1*2 + 0*4 + 2*6 + 20 = 34
        let m = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let out = matvec(&m, 3, 2, &[1.0, 0.0, 2.0], &[10.0, 20.0]);
        assert_eq!(out, vec![21.0, 34.0]);
    }

    #[test]
    fn sigmoid_is_centered() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-6);
        assert!(sigmoid(50.0) > 0.999);
        assert!(sigmoid(-50.0) < 0.001);
    }

    fn tiny_weights() -> MfWeights {
        // d=2, num_classes=2, no projection (text_dim==d).
        // cls_w identity-ish: interaction maps directly to logits.
        MfWeights {
            version: super::super::weights::MF_WEIGHTS_VERSION,
            embedding_model: "test".into(),
            text_dim: 2,
            d: 2,
            num_classes: 2,
            strong_row: vec![1.0, 0.0], // normalizes to [1,0]
            weak_row: vec![0.0, 1.0],   // normalizes to [0,1]
            use_proj: false,
            proj_w: None,
            proj_b: None,
            // cls_w row-major [d*num_classes] = [[w00,w01],[w10,w11]]
            // logits[c] = sum_i interaction[i]*cls_w[i*2 + c]
            cls_w: vec![1.0, 0.0, 0.0, 1.0],
            cls_b: vec![0.0, 0.0],
            strong_class: 0,
            weak_class: 1,
        }
    }

    #[test]
    fn mf_hardness_matches_hand_computation() {
        let w = tiny_weights();
        let e = vec![2.0, 3.0];
        // strong_norm = [1,0]; pe = [2,3]; interaction_strong = [2,0]
        //   logits_strong = [2*1+0*0, 2*0+0*1] = [2,0]; strong_class=0 -> 2
        // weak_norm = [0,1]; interaction_weak = [0,3]
        //   logits_weak = [0*1+3*0, 0*0+3*1] = [0,3]; weak_class=1 -> 3
        // diff = 2 - 3 = -1 ; sigmoid(-1) ~ 0.26894
        let s = mf_hardness(&w, &e).unwrap();
        assert!((s - 0.268941).abs() < 1e-4, "s was {s}");
    }

    #[test]
    fn mf_hardness_with_projection() {
        let mut w = tiny_weights();
        // text_dim=3 -> d=2 projection (identity on first two dims).
        w.text_dim = 3;
        w.use_proj = true;
        // proj_w row-major [text_dim*d] = 3x2
        w.proj_w = Some(vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        w.proj_b = Some(vec![0.0, 0.0]);
        let e = vec![2.0, 3.0, 99.0]; // third dim dropped by projection
        let s = mf_hardness(&w, &e).unwrap();
        assert!((s - 0.268941).abs() < 1e-4, "s was {s}");
    }

    #[test]
    fn mf_hardness_rejects_wrong_dim() {
        let w = tiny_weights();
        assert!(mf_hardness(&w, &[1.0, 2.0, 3.0]).is_err());
    }
}
