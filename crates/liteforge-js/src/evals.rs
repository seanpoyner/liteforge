use napi::bindgen_prelude::*;
use liteforge::evals::{
    ContainsEvaluator, EvalResult as RustEvalResult, EvalSuite as RustEvalSuite,
    ExactMatchEvaluator, JsonMatchEvaluator, RegexEvaluator, SimilarityEvaluator,
    SuiteResult as RustSuiteResult, SuiteStats as RustSuiteStats, TestCase as RustTestCase,
};

#[napi(object)]
pub struct JsTestCase {
    pub input: String,
    pub expected: String,
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub weight: f64,
}

#[napi(object)]
pub struct JsEvalResult {
    pub passed: bool,
    pub score: f64,
    pub reason: Option<String>,
}

fn rust_eval_result_to_js(r: &RustEvalResult) -> JsEvalResult {
    JsEvalResult {
        passed: r.passed,
        score: r.score,
        reason: r.reason.clone(),
    }
}

#[napi(object)]
pub struct JsCaseResult {
    pub input: String,
    pub expected: String,
    pub actual: String,
    pub eval: JsEvalResult,
    pub duration_ms: u32,
}

#[napi(object)]
pub struct JsSuiteStats {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub pass_rate: f64,
    pub avg_score: f64,
    pub weighted_avg_score: f64,
    pub duration_ms: u32,
}

fn rust_suite_stats_to_js(s: &RustSuiteStats) -> JsSuiteStats {
    JsSuiteStats {
        total: s.total as u32,
        passed: s.passed as u32,
        failed: s.failed as u32,
        pass_rate: s.pass_rate,
        avg_score: s.avg_score,
        weighted_avg_score: s.weighted_avg_score,
        duration_ms: s.duration_ms as u32,
    }
}

#[napi(object)]
pub struct JsSuiteResult {
    pub name: String,
    pub evaluator: String,
    pub results: Vec<JsCaseResult>,
    pub stats: JsSuiteStats,
}

fn rust_suite_result_to_js(r: &RustSuiteResult) -> JsSuiteResult {
    JsSuiteResult {
        name: r.name.clone(),
        evaluator: r.evaluator.clone(),
        results: r
            .results
            .iter()
            .map(|cr| JsCaseResult {
                input: cr.case.input.clone(),
                expected: cr.case.expected.clone(),
                actual: cr.actual.clone(),
                eval: rust_eval_result_to_js(&cr.eval),
                duration_ms: cr.duration_ms as u32,
            })
            .collect(),
        stats: rust_suite_stats_to_js(&r.stats),
    }
}

#[napi]
pub struct EvalSuite {
    inner: RustEvalSuite,
}

#[napi]
impl EvalSuite {
    #[napi(constructor)]
    pub fn new(name: String) -> Self {
        Self {
            inner: RustEvalSuite::new(name),
        }
    }

    #[napi]
    pub fn add_case(&mut self, input: String, expected: String) {
        let tc = RustTestCase::new(input, expected);
        self.inner.cases.push(tc);
    }

    #[napi]
    pub fn add_named_case(&mut self, input: String, expected: String, name: String) {
        let tc = RustTestCase::new(input, expected).name(name);
        self.inner.cases.push(tc);
    }

    #[napi]
    pub fn run_exact_match(&self, outputs: Vec<String>) -> Result<JsSuiteResult> {
        let evaluator = ExactMatchEvaluator::new();
        let result = self.inner.run(&evaluator, &outputs);
        Ok(rust_suite_result_to_js(&result))
    }

    #[napi]
    pub fn run_contains(&self, outputs: Vec<String>) -> Result<JsSuiteResult> {
        let evaluator = ContainsEvaluator::new();
        let result = self.inner.run(&evaluator, &outputs);
        Ok(rust_suite_result_to_js(&result))
    }

    #[napi]
    pub fn run_regex(&self, outputs: Vec<String>) -> Result<JsSuiteResult> {
        let evaluator = RegexEvaluator::new();
        let result = self.inner.run(&evaluator, &outputs);
        Ok(rust_suite_result_to_js(&result))
    }

    #[napi]
    pub fn run_similarity(&self, outputs: Vec<String>, threshold: f64) -> Result<JsSuiteResult> {
        let evaluator = SimilarityEvaluator::new(threshold);
        let result = self.inner.run(&evaluator, &outputs);
        Ok(rust_suite_result_to_js(&result))
    }

    #[napi]
    pub fn run_json_match(&self, outputs: Vec<String>) -> Result<JsSuiteResult> {
        let evaluator = JsonMatchEvaluator::new();
        let result = self.inner.run(&evaluator, &outputs);
        Ok(rust_suite_result_to_js(&result))
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
