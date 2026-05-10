//! Metrics collection for monitoring.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// A metric value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// A counter (monotonically increasing).
    Counter(u64),
    /// A gauge (can go up or down).
    Gauge(f64),
    /// A histogram of values.
    Histogram(HistogramData),
}

/// Histogram data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramData {
    /// Number of observations.
    pub count: u64,
    /// Sum of all observations.
    pub sum: f64,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// Bucket counts (for percentile calculation).
    pub buckets: Vec<(f64, u64)>,
}

impl Default for HistogramData {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            buckets: Vec::new(),
        }
    }
}

impl HistogramData {
    /// Create a new histogram with default buckets.
    pub fn new() -> Self {
        Self::with_buckets(&[1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
    }

    /// Create a histogram with custom bucket boundaries.
    pub fn with_buckets(boundaries: &[f64]) -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            buckets: boundaries.iter().map(|&b| (b, 0)).collect(),
        }
    }

    /// Record a value.
    pub fn record(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);

        // Update bucket counts
        for (boundary, count) in &mut self.buckets {
            if value <= *boundary {
                *count += 1;
            }
        }
    }

    /// Get the mean value.
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Estimate a percentile (0.0-1.0).
    pub fn percentile(&self, p: f64) -> f64 {
        if self.count == 0 || self.buckets.is_empty() {
            return 0.0;
        }

        let target = (self.count as f64 * p) as u64;
        let mut prev_boundary = 0.0;
        let mut prev_count = 0u64;

        for (boundary, count) in &self.buckets {
            if *count >= target {
                // Linear interpolation within the bucket
                if *count == prev_count {
                    return *boundary;
                }
                let ratio = (target - prev_count) as f64 / (*count - prev_count) as f64;
                return prev_boundary + ratio * (*boundary - prev_boundary);
            }
            prev_boundary = *boundary;
            prev_count = *count;
        }

        self.max
    }

    /// Get the p50 (median).
    pub fn p50(&self) -> f64 {
        self.percentile(0.5)
    }

    /// Get the p90.
    pub fn p90(&self) -> f64 {
        self.percentile(0.9)
    }

    /// Get the p99.
    pub fn p99(&self) -> f64 {
        self.percentile(0.99)
    }
}

/// Internal metric storage.
#[derive(Debug)]
struct MetricData {
    value: MetricValue,
    #[allow(dead_code)]
    labels: HashMap<String, String>,
    last_updated: Instant,
}

/// Metrics collector for gathering metrics.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    metrics: Arc<Mutex<HashMap<String, MetricData>>>,
    prefix: Option<String>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            prefix: None,
        }
    }

    /// Create a metrics collector with a prefix.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            prefix: Some(prefix.into()),
        }
    }

    /// Get the full metric name with prefix.
    fn full_name(&self, name: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{}_{}", prefix, name),
            None => name.to_string(),
        }
    }

    /// Increment a counter.
    pub fn increment(&self, name: &str, value: u64) {
        self.increment_with_labels(name, value, HashMap::new());
    }

    /// Increment a counter with labels.
    pub fn increment_with_labels(&self, name: &str, value: u64, labels: HashMap<String, String>) {
        let full_name = self.full_name(name);
        let mut metrics = self.metrics.lock().unwrap();

        let entry = metrics.entry(full_name).or_insert_with(|| MetricData {
            value: MetricValue::Counter(0),
            labels: labels.clone(),
            last_updated: Instant::now(),
        });

        if let MetricValue::Counter(ref mut v) = entry.value {
            *v += value;
        }
        entry.last_updated = Instant::now();
    }

    /// Set a gauge value.
    pub fn gauge(&self, name: &str, value: f64) {
        self.gauge_with_labels(name, value, HashMap::new());
    }

    /// Set a gauge value with labels.
    pub fn gauge_with_labels(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let full_name = self.full_name(name);
        let mut metrics = self.metrics.lock().unwrap();

        let entry = metrics.entry(full_name).or_insert_with(|| MetricData {
            value: MetricValue::Gauge(0.0),
            labels: labels.clone(),
            last_updated: Instant::now(),
        });

        entry.value = MetricValue::Gauge(value);
        entry.last_updated = Instant::now();
    }

    /// Record a duration in milliseconds.
    pub fn record_duration(&self, name: &str, duration_ms: u64) {
        self.record_histogram(name, duration_ms as f64);
    }

    /// Record a value in a histogram.
    pub fn record_histogram(&self, name: &str, value: f64) {
        self.record_histogram_with_labels(name, value, HashMap::new());
    }

    /// Record a histogram value with labels.
    pub fn record_histogram_with_labels(
        &self,
        name: &str,
        value: f64,
        labels: HashMap<String, String>,
    ) {
        let full_name = self.full_name(name);
        let mut metrics = self.metrics.lock().unwrap();

        let entry = metrics.entry(full_name).or_insert_with(|| MetricData {
            value: MetricValue::Histogram(HistogramData::new()),
            labels: labels.clone(),
            last_updated: Instant::now(),
        });

        if let MetricValue::Histogram(ref mut h) = entry.value {
            h.record(value);
        }
        entry.last_updated = Instant::now();
    }

    /// Time a closure and record its duration.
    pub fn time<T, F: FnOnce() -> T>(&self, name: &str, f: F) -> T {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        self.record_duration(name, duration.as_millis() as u64);
        result
    }

    /// Time an async closure and record its duration.
    pub async fn time_async<T, F, Fut>(&self, name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        self.record_duration(name, duration.as_millis() as u64);
        result
    }

    /// Get a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let metrics = self.metrics.lock().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut values = HashMap::new();
        for (name, data) in metrics.iter() {
            values.insert(name.clone(), data.value.clone());
        }

        MetricsSnapshot { timestamp, values }
    }

    /// Get a specific metric value.
    pub fn get(&self, name: &str) -> Option<MetricValue> {
        let full_name = self.full_name(name);
        let metrics = self.metrics.lock().unwrap();
        metrics.get(&full_name).map(|d| d.value.clone())
    }

    /// Get the number of metrics.
    pub fn count(&self) -> usize {
        self.metrics.lock().unwrap().len()
    }

    /// Clear all metrics.
    pub fn clear(&self) {
        self.metrics.lock().unwrap().clear();
    }

    /// List all metric names.
    pub fn names(&self) -> Vec<String> {
        self.metrics.lock().unwrap().keys().cloned().collect()
    }
}

/// A snapshot of metrics at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp (Unix epoch milliseconds).
    pub timestamp: u64,
    /// Metric values.
    pub values: HashMap<String, MetricValue>,
}

impl MetricsSnapshot {
    /// Get a counter value.
    pub fn counter(&self, name: &str) -> Option<u64> {
        match self.values.get(name) {
            Some(MetricValue::Counter(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a gauge value.
    pub fn gauge(&self, name: &str) -> Option<f64> {
        match self.values.get(name) {
            Some(MetricValue::Gauge(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get histogram data.
    pub fn histogram(&self, name: &str) -> Option<&HistogramData> {
        match self.values.get(name) {
            Some(MetricValue::Histogram(h)) => Some(h),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counter() {
        let metrics = MetricsCollector::new();

        metrics.increment("requests", 1);
        metrics.increment("requests", 1);
        metrics.increment("requests", 3);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.counter("requests"), Some(5));
    }

    #[test]
    fn test_metrics_gauge() {
        let metrics = MetricsCollector::new();

        metrics.gauge("temperature", 23.5);
        assert_eq!(metrics.snapshot().gauge("temperature"), Some(23.5));

        metrics.gauge("temperature", 24.0);
        assert_eq!(metrics.snapshot().gauge("temperature"), Some(24.0));
    }

    #[test]
    fn test_metrics_histogram() {
        let metrics = MetricsCollector::new();

        for i in 1..=100 {
            metrics.record_histogram("latency", i as f64);
        }

        let snapshot = metrics.snapshot();
        let hist = snapshot.histogram("latency").unwrap();

        assert_eq!(hist.count, 100);
        assert_eq!(hist.min, 1.0);
        assert_eq!(hist.max, 100.0);
        assert_eq!(hist.mean(), 50.5);
    }

    #[test]
    fn test_metrics_with_prefix() {
        let metrics = MetricsCollector::with_prefix("myapp");

        metrics.increment("requests", 1);

        let snapshot = metrics.snapshot();
        assert!(snapshot.values.contains_key("myapp_requests"));
    }

    #[test]
    fn test_metrics_time() {
        let metrics = MetricsCollector::new();

        let result = metrics.time("operation_duration", || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);

        let snapshot = metrics.snapshot();
        let hist = snapshot.histogram("operation_duration").unwrap();
        assert!(hist.mean() >= 10.0);
    }

    #[test]
    fn test_histogram_percentiles() {
        let mut hist = HistogramData::with_buckets(&[10.0, 50.0, 100.0, 500.0, 1000.0]);

        // Add values: mostly small, some large
        for _ in 0..90 {
            hist.record(5.0);
        }
        for _ in 0..9 {
            hist.record(75.0);
        }
        hist.record(999.0);

        assert_eq!(hist.count, 100);
        assert!(hist.p50() <= 10.0); // 50% of values are <= 10
        assert!(hist.p90() <= 50.0); // 90% of values are <= 50 (actually 5.0)
        assert!(hist.p99() <= 100.0); // 99% of values are <= 100
    }

    #[test]
    fn test_metrics_list_names() {
        let metrics = MetricsCollector::new();

        metrics.increment("counter1", 1);
        metrics.gauge("gauge1", 1.0);
        metrics.record_histogram("hist1", 1.0);

        let names = metrics.names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"counter1".to_string()));
        assert!(names.contains(&"gauge1".to_string()));
        assert!(names.contains(&"hist1".to_string()));
    }

    #[tokio::test]
    async fn test_metrics_time_async() {
        let metrics = MetricsCollector::new();

        let result = metrics
            .time_async("async_operation", || async {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                42
            })
            .await;

        assert_eq!(result, 42);

        let snapshot = metrics.snapshot();
        let hist = snapshot.histogram("async_operation").unwrap();
        assert!(hist.mean() >= 10.0);
    }
}
