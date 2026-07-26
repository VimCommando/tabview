use std::fmt;
use std::num::NonZeroUsize;
use std::sync::{mpsc, Arc, Condvar, Mutex};

use super::{CellValue, ColumnId, SourceGeneration, SourceQueryTask, TableStore};

#[cfg(feature = "sqlite")]
pub const DEFAULT_SQLITE_SOURCE_LIMIT: usize = 1_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NullPlacement {
    First,
    #[default]
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueDomain {
    Raw,
    Rendered,
    RawOrRendered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    In,
    Out,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewFilterPredicate {
    Text {
        value: String,
        domain: ValueDomain,
    },
    Regex {
        pattern: String,
        domain: ValueDomain,
    },
    Numeric {
        operator: NumericOperator,
        operand: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewFilter {
    pub column: ColumnId,
    pub mode: FilterMode,
    pub predicate: ViewFilterPredicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewSortMode {
    Lexical,
    Natural,
    Numeric,
    Date,
    SemanticVersion,
    Ip,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewSort {
    pub column: ColumnId,
    pub mode: ViewSortMode,
    pub direction: SortDirection,
    pub nulls: NullPlacement,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewTransform {
    pub generation: SourceGeneration,
    pub filters: Vec<ViewFilter>,
    pub order_by: Vec<ViewSort>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceOperand {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Binary(Vec<u8>),
}

impl From<SourceOperand> for CellValue {
    fn from(value: SourceOperand) -> Self {
        match value {
            SourceOperand::Null => Self::Null,
            SourceOperand::Boolean(value) => Self::Boolean(value),
            SourceOperand::Integer(value) => Self::Integer(value),
            SourceOperand::Float(value) => Self::Float(value),
            SourceOperand::Text(value) => Self::Text(value),
            SourceOperand::Binary(value) => Self::Binary(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceFilterOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Contains,
    Prefix,
    IsNull,
    IsNotNull,
}

impl SourceFilterOperator {
    pub fn requires_operand(self) -> bool {
        !matches!(self, Self::IsNull | Self::IsNotNull)
    }
}

impl fmt::Display for SourceFilterOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Equal => "equal",
            Self::NotEqual => "not equal",
            Self::LessThan => "less than",
            Self::LessThanOrEqual => "less than or equal",
            Self::GreaterThan => "greater than",
            Self::GreaterThanOrEqual => "greater than or equal",
            Self::Contains => "contains",
            Self::Prefix => "prefix",
            Self::IsNull => "is null",
            Self::IsNotNull => "is not null",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceFilterScope {
    WholeRecord,
    Column(ColumnId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFilter {
    pub scope: SourceFilterScope,
    pub operator: SourceFilterOperator,
    pub operand: Option<SourceOperand>,
}

impl SourceFilter {
    pub fn validate(&self) -> Result<(), SourceQueryValidationError> {
        if self.operator.requires_operand() != self.operand.is_some() {
            return Err(SourceQueryValidationError::OperandMismatch {
                operator: self.operator,
            });
        }
        if matches!(self.operand, Some(SourceOperand::Float(value)) if !value.is_finite()) {
            return Err(SourceQueryValidationError::NonFiniteOperand);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSort {
    pub column: ColumnId,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceQuery {
    pub generation: SourceGeneration,
    pub filters: Vec<SourceFilter>,
    pub order_by: Vec<SourceSort>,
    pub limit: NonZeroUsize,
}

impl SourceQuery {
    pub fn new(generation: SourceGeneration, limit: NonZeroUsize) -> Self {
        Self {
            generation,
            filters: Vec::new(),
            order_by: Vec::new(),
            limit,
        }
    }

    #[cfg(feature = "sqlite")]
    pub fn sqlite_default(generation: SourceGeneration) -> Self {
        Self::new(
            generation,
            NonZeroUsize::new(DEFAULT_SQLITE_SOURCE_LIMIT).expect("non-zero SQLite default"),
        )
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SourceQueryValidationError {
    #[error("source query belongs to a different source generation")]
    StaleGeneration,
    #[error("source query references an unknown or stale column")]
    UnknownColumn,
    #[error("source filter '{operator}' has the wrong operand shape")]
    OperandMismatch { operator: SourceFilterOperator },
    #[error("source query contains a non-finite floating-point operand")]
    NonFiniteOperand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityStatus {
    Supported,
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceQueryProgress {
    Idle,
    Pending { revision: u64 },
    Failed { revision: u64, error: String },
}

pub enum SourceQueryCoordinatorEvent {
    Ready {
        revision: u64,
        store: Box<dyn TableStore>,
    },
    Failed {
        revision: u64,
        error: String,
    },
}

struct SourceQueryJobResult {
    revision: u64,
    result: anyhow::Result<Box<dyn TableStore>>,
}

struct SourceQueryJob {
    revision: u64,
    task: SourceQueryTask,
}

#[derive(Default)]
struct SourceQueryWorkerState {
    pending: Option<SourceQueryJob>,
    shutdown: bool,
}

pub struct SourceQueryCoordinator {
    next_revision: u64,
    latest_requested: u64,
    progress: SourceQueryProgress,
    worker: Arc<(Mutex<SourceQueryWorkerState>, Condvar)>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
    receiver: mpsc::Receiver<SourceQueryJobResult>,
}

impl fmt::Debug for SourceQueryCoordinator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceQueryCoordinator")
            .field("latest_requested", &self.latest_requested)
            .field("progress", &self.progress)
            .finish_non_exhaustive()
    }
}

impl Default for SourceQueryCoordinator {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = Arc::new((
            Mutex::new(SourceQueryWorkerState::default()),
            Condvar::new(),
        ));
        let worker_state = worker.clone();
        let worker_handle = std::thread::Builder::new()
            .name("tabview-source-query".to_owned())
            .spawn(move || source_query_worker(worker_state, sender))
            .expect("source query worker thread");
        Self {
            next_revision: 1,
            latest_requested: 0,
            progress: SourceQueryProgress::Idle,
            worker,
            worker_handle: Some(worker_handle),
            receiver,
        }
    }
}

impl SourceQueryCoordinator {
    pub fn request(&mut self, task: SourceQueryTask) -> u64 {
        let revision = self.next_revision;
        self.next_revision = self.next_revision.saturating_add(1);
        self.latest_requested = revision;
        self.progress = SourceQueryProgress::Pending { revision };
        let (state, wake) = &*self.worker;
        let mut state = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.pending = Some(SourceQueryJob { revision, task });
        wake.notify_one();
        revision
    }

    pub fn progress(&self) -> &SourceQueryProgress {
        &self.progress
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.progress, SourceQueryProgress::Pending { .. })
    }

    pub fn poll(&mut self) -> Option<SourceQueryCoordinatorEvent> {
        let mut latest = None;
        while let Ok(result) = self.receiver.try_recv() {
            if result.revision == self.latest_requested {
                latest = Some(result);
            }
        }
        let result = latest?;
        match result.result {
            Ok(store) => {
                self.progress = SourceQueryProgress::Idle;
                Some(SourceQueryCoordinatorEvent::Ready {
                    revision: result.revision,
                    store,
                })
            }
            Err(error) => {
                let error = error.to_string();
                self.progress = SourceQueryProgress::Failed {
                    revision: result.revision,
                    error: error.clone(),
                };
                Some(SourceQueryCoordinatorEvent::Failed {
                    revision: result.revision,
                    error,
                })
            }
        }
    }
}

impl Drop for SourceQueryCoordinator {
    fn drop(&mut self) {
        let (state, wake) = &*self.worker;
        let mut state = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.shutdown = true;
        state.pending = None;
        wake.notify_one();
        // Do not block application shutdown on an active file scan. The one
        // worker exits as soon as its current task returns.
        self.worker_handle.take();
    }
}

fn source_query_worker(
    worker: Arc<(Mutex<SourceQueryWorkerState>, Condvar)>,
    sender: mpsc::Sender<SourceQueryJobResult>,
) {
    loop {
        let job = {
            let (state, wake) = &*worker;
            let mut state = state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            while state.pending.is_none() && !state.shutdown {
                state = wake
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            if state.shutdown {
                return;
            }
            state.pending.take().expect("pending source query job")
        };
        let result = (job.task)();
        if sender
            .send(SourceQueryJobResult {
                revision: job.revision,
                result,
            })
            .is_err()
        {
            return;
        }
    }
}

impl CapabilityStatus {
    pub fn is_supported(&self) -> bool {
        matches!(self, Self::Supported)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOperationCapabilities {
    pub filters: Vec<SourceFilterOperator>,
    pub sorting: CapabilityStatus,
    pub configurable_limit: bool,
}

impl SourceOperationCapabilities {
    pub fn supports_filter(&self, operator: SourceFilterOperator) -> bool {
        self.filters.contains(&operator)
    }
}

impl Default for SourceOperationCapabilities {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            sorting: CapabilityStatus::Unavailable {
                reason: "source-native sorting is unavailable for this source".to_owned(),
            },
            configurable_limit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_layers_reference_generation_scoped_columns() {
        let generation = SourceGeneration::new();
        let column = ColumnId {
            generation,
            ordinal: 1,
        };
        let view = ViewTransform {
            generation,
            filters: vec![ViewFilter {
                column,
                mode: FilterMode::In,
                predicate: ViewFilterPredicate::Text {
                    value: "ok".to_owned(),
                    domain: ValueDomain::RawOrRendered,
                },
            }],
            order_by: vec![ViewSort {
                column,
                mode: ViewSortMode::Natural,
                direction: SortDirection::Descending,
                nulls: NullPlacement::First,
            }],
        };
        let source = SourceQuery {
            generation,
            filters: vec![SourceFilter {
                scope: SourceFilterScope::Column(column),
                operator: SourceFilterOperator::Equal,
                operand: Some(SourceOperand::Text("ok".to_owned())),
            }],
            order_by: vec![SourceSort {
                column,
                direction: SortDirection::Ascending,
            }],
            limit: NonZeroUsize::new(1_000).unwrap(),
        };
        assert_eq!(view.order_by[0].nulls, NullPlacement::First);
        assert!(matches!(
            source.filters[0].scope,
            SourceFilterScope::Column(id) if id.generation == generation
        ));
    }

    #[test]
    fn null_tests_reject_operands_and_comparisons_require_them() {
        let generation = SourceGeneration::new();
        let column = ColumnId {
            generation,
            ordinal: 0,
        };
        assert!(SourceFilter {
            scope: SourceFilterScope::Column(column),
            operator: SourceFilterOperator::IsNull,
            operand: None,
        }
        .validate()
        .is_ok());
        assert!(SourceFilter {
            scope: SourceFilterScope::Column(column),
            operator: SourceFilterOperator::Equal,
            operand: None,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn coordinator_publishes_only_latest_revision() {
        let generation = SourceGeneration::new();
        let mut coordinator = SourceQueryCoordinator::default();
        let (release, wait) = std::sync::mpsc::channel();
        let (started, first_started) = std::sync::mpsc::channel();
        let superseded_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        coordinator.request(Box::new(move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
            Ok(Box::new(super::super::InMemoryTable::from_text_rows(
                generation,
                vec![vec!["stale".to_owned()]],
            )))
        }));
        first_started
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("first source query started");
        let ran = superseded_ran.clone();
        coordinator.request(Box::new(move || {
            ran.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(Box::new(super::super::InMemoryTable::from_text_rows(
                generation,
                vec![vec!["superseded".to_owned()]],
            )))
        }));
        let latest = coordinator.request(Box::new(move || {
            Ok(Box::new(super::super::InMemoryTable::from_text_rows(
                generation,
                vec![vec!["latest".to_owned()]],
            )))
        }));
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let event = loop {
            if let Some(event) = coordinator.poll() {
                break event;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "source query worker timed out"
            );
            std::thread::yield_now();
        };
        assert!(matches!(
            event,
            SourceQueryCoordinatorEvent::Ready { revision, .. } if revision == latest
        ));
        assert!(!superseded_ran.load(std::sync::atomic::Ordering::SeqCst));
        assert!(coordinator.poll().is_none());
    }
}
