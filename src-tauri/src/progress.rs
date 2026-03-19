use serde::Serialize;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub const UPLOAD_PROGRESS_EVENT: &str = "upload-progress";
pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransferStage {
    Starting,
    Connected,
    Progress,
    Saving,
    Finished,
    Error,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub stage: TransferStage,
    pub message: Option<String>,
    pub bytes_done: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f64>,
    pub speed_bps: Option<f64>,
    pub eta_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressSnapshot {
    pub bytes_done: u64,
    pub total_bytes: Option<u64>,
    pub elapsed: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressMetrics {
    pub percent: Option<f64>,
    pub speed_bps: Option<f64>,
    pub eta_seconds: Option<u64>,
}

pub trait ProgressMetricsCalculator: Send + Sync {
    fn calculate(&self, snapshot: ProgressSnapshot) -> ProgressMetrics;
}

#[derive(Default)]
pub struct LinearProgressMetricsCalculator;

impl ProgressMetricsCalculator for LinearProgressMetricsCalculator {
    fn calculate(&self, snapshot: ProgressSnapshot) -> ProgressMetrics {
        let percent = snapshot
            .total_bytes
            .filter(|total| *total > 0)
            .map(|total| (snapshot.bytes_done as f64 / total as f64) * 100.0);
        let speed_bps =
            (snapshot.elapsed.as_secs_f64() > 0.0).then_some(snapshot.bytes_done as f64 / snapshot.elapsed.as_secs_f64());
        let eta_seconds = match (speed_bps, snapshot.total_bytes) {
            (Some(speed), Some(total)) if speed > 0.0 && snapshot.bytes_done < total => {
                Some(((total - snapshot.bytes_done) as f64 / speed).ceil() as u64)
            }
            _ => None,
        };

        ProgressMetrics {
            percent,
            speed_bps,
            eta_seconds,
        }
    }
}

pub trait ProgressReporter: Send + Sync {
    fn report(&self, progress: TransferProgress);
}

pub struct ProgressTracker<C = LinearProgressMetricsCalculator> {
    started_at: Instant,
    fixed_total_bytes: Option<u64>,
    item_sizes: HashMap<u64, u64>,
    item_offsets: HashMap<u64, u64>,
    calculator: C,
}

impl<C> ProgressTracker<C>
where
    C: ProgressMetricsCalculator,
{
    pub fn new(fixed_total_bytes: Option<u64>, calculator: C) -> Self {
        Self {
            started_at: Instant::now(),
            fixed_total_bytes,
            item_sizes: HashMap::new(),
            item_offsets: HashMap::new(),
            calculator,
        }
    }

    pub fn register_item(&mut self, id: u64, size: u64) {
        self.item_sizes.insert(id, size);
        self.item_offsets.entry(id).or_insert(0);
    }

    pub fn mark_progress(&mut self, id: u64, offset: u64) {
        self.item_offsets.insert(id, offset);
    }

    pub fn mark_complete(&mut self, id: u64) {
        if let Some(size) = self.item_sizes.get(&id).copied() {
            self.item_offsets.insert(id, size);
        }
    }

    pub fn mark_local_complete(&mut self, id: u64, size: u64) {
        self.item_sizes.insert(id, size);
        self.item_offsets.insert(id, size);
    }

    pub fn snapshot(
        &self,
        stage: TransferStage,
        message: impl Into<Option<String>>,
    ) -> TransferProgress {
        let bytes_done = self.bytes_done();
        let total_bytes = self.total_bytes();
        let metrics = self.calculator.calculate(ProgressSnapshot {
            bytes_done,
            total_bytes,
            elapsed: self.started_at.elapsed(),
        });

        TransferProgress {
            stage,
            message: message.into(),
            bytes_done,
            total_bytes,
            percent: metrics.percent,
            speed_bps: metrics.speed_bps,
            eta_seconds: metrics.eta_seconds,
        }
    }

    fn bytes_done(&self) -> u64 {
        self.item_offsets
            .iter()
            .map(|(id, offset)| {
                let size = self.item_sizes.get(id).copied().unwrap_or(*offset);
                (*offset).min(size)
            })
            .sum()
    }

    fn total_bytes(&self) -> Option<u64> {
        self.fixed_total_bytes.or_else(|| {
            if self.item_sizes.is_empty() {
                None
            } else {
                Some(self.item_sizes.values().sum())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedCalculator;

    impl ProgressMetricsCalculator for FixedCalculator {
        fn calculate(&self, _snapshot: ProgressSnapshot) -> ProgressMetrics {
            ProgressMetrics {
                percent: Some(42.0),
                speed_bps: Some(512.0),
                eta_seconds: Some(3),
            }
        }
    }

    #[test]
    fn tracker_accepts_custom_metric_calculators() {
        let mut tracker = ProgressTracker::new(Some(100), FixedCalculator);
        tracker.register_item(1, 100);
        tracker.mark_progress(1, 20);

        let snapshot = tracker.snapshot(TransferStage::Progress, Some("test".to_string()));

        assert_eq!(snapshot.percent, Some(42.0));
        assert_eq!(snapshot.speed_bps, Some(512.0));
        assert_eq!(snapshot.eta_seconds, Some(3));
    }

    #[test]
    fn linear_calculator_uses_elapsed_time_and_total_size() {
        let metrics = LinearProgressMetricsCalculator.calculate(ProgressSnapshot {
            bytes_done: 50,
            total_bytes: Some(100),
            elapsed: Duration::from_secs(5),
        });

        assert_eq!(metrics.percent, Some(50.0));
        assert_eq!(metrics.speed_bps, Some(10.0));
        assert_eq!(metrics.eta_seconds, Some(5));
    }
}
