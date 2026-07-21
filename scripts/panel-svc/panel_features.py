"""Shared structured-feature extractor for the panel router.

Used by both the fusion trainer and the serving app so training and inference
compute identical features from the same text. Features are coarse and robust:
they describe the size and shape of the codebase context attached to a prompt.
"""
import re

CTX_TOKEN_SCALE = 2000.0
N_FILES_SCALE = 8.0
STRUCT_FEATURES = ["ctx_tokens", "n_files", "has_code", "has_diff", "has_error"]

_FILE_LINE = re.compile(r"(?m)^\s*[-*]?\s*(?:src/|\./)?[\w./-]+\.[A-Za-z]{1,4}\b")
_DIFF = re.compile(r"diff --git|^@@ ", re.MULTILINE)
_ERROR = re.compile(r"Traceback|Exception|panic!|\bError:|\bpanic\b|stack trace", re.IGNORECASE)
_CODE_KW = re.compile(r"```|^\s{4}\S|\bdef \b|\bfn \b|\bfunction \b|\bclass \b|\bimport \b|SELECT\s", re.MULTILINE)


def extract_features(text: str) -> dict:
    has_diff = 1 if _DIFF.search(text) else 0
    has_error = 1 if _ERROR.search(text) else 0
    has_code = 1 if (_CODE_KW.search(text) or "```" in text) else 0
    # Count distinct file-path-looking tokens (cap is applied at normalization).
    files = set(m.group(0).strip("-* ").strip() for m in _FILE_LINE.finditer(text))
    n_files = len(files)
    ctx_tokens = max(1, len(text) // 4)
    return {
        "ctx_tokens": ctx_tokens,
        "n_files": n_files,
        "has_code": has_code,
        "has_diff": has_diff,
        "has_error": has_error,
    }


def norm_struct(feats: dict) -> list:
    return [
        min(feats["ctx_tokens"] / CTX_TOKEN_SCALE, 1.0),
        min(feats["n_files"] / N_FILES_SCALE, 1.0),
        float(feats["has_code"]),
        float(feats["has_diff"]),
        float(feats["has_error"]),
    ]
