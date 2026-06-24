# Router evaluation harness

Benchmarks the LiteForge routers against [RouterBench](https://huggingface.co/datasets/withmartian/routerbench)
(precomputed per-model correctness + cost, so no LLM inference is needed) plus an
intrinsic out-of-distribution check and a small live sample.

## Setup

```bash
pip install --user pyarrow matplotlib   # pandas/numpy/sklearn/transformers/torch assumed
export LITEFORGE_API_KEY=...            # for bge-m3 embeddings (MF + embedding-head retrain)
```

Trained router artifacts are read from `~/.forge/router-models/` (panel, router-bert)
and `~/.forge/mf_weights.bge-m3.json` (MF); override with `ROUTER_MODELS`.

## Run

```bash
python fetch_data.py                       # download RouterBench, build intrinsic_sample.jsonl
python intrinsic_eval.py                   # task/difficulty OOD accuracy + confusion
python routerbench_eval.py --n 2000 --with-mf --tag zeroshot   # cost-quality, zero-shot
python retrain_on_routerbench.py           # from-scratch bert-mini baseline (negative result)
python retrain_emb.py                      # bge-m3 logistic head (the strong small router)
python live_sample.py                      # small live end-to-end through router-panel
python plots.py                            # figures for the paper
```

Results land in `results/*.json` and `results/figures/*.png`, consumed by
`../../docs/paper/liteforge-routing.tex`.

## Key findings (RouterBench, strong=gpt-4 / weak=mixtral-8x7b)

| Router | Training | APGR (0=random,1=oracle) | Cost saved @95% strong |
|---|---|---|---|
| router-bert (difficulty) | synthetic | -0.36 | 7.5% |
| panel (difficulty signal) | synthetic | -0.03 | 10.4% |
| MF port | synthetic + bge-m3 | +0.10 | 8.6% |
| bert-mini (from scratch) | RouterBench | -0.11 | 10.2% |
| **bge-m3 logistic head** | **RouterBench** | **+0.31** | **21.7%** |

Takeaway: synthetic-trained and from-scratch tiny routers transfer poorly; a tiny
head over strong pretrained embeddings is what works. See the paper for details.
