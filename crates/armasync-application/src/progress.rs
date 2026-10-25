// SPDX-License-Identifier: AGPL-3.0-only
//! Upload events of a publication (MIG-042, step 4c; ADR-0016).
//!
//! [`UploadObserver`] owns the sink of one operation. It numbers the events, keeps the counters and guarantees the
//! order: exactly one `Started` first, then the files, then exactly one terminal event, after which nothing is sent.
//! The observer is shared through an `Arc`, because the bytes of a running file are reported from inside the reader
//! that the adapter consumes.

use std::fmt;
use std::io::{self, Read};
use std::sync::{Arc, Mutex, MutexGuard};

use armasync_core::{
    Error, ErrorKind, OperationId, RelativePath, UPLOAD_EVENT_SCHEMA_VERSION, UploadEvent,
    UploadEventKind, UploadEventSink, UploadProgress,
};

/// Reports the events of one publication to a sink. Cloning shares the same sequence and counters.
#[derive(Clone)]
pub struct UploadObserver {
    inner: Arc<Mutex<State>>,
}

struct State {
    sink: Box<dyn UploadEventSink>,
    operation: OperationId,
    sequence: u64,
    progress: UploadProgress,
    started: bool,
    finished: bool,
    file_size: u64,
    file_base: u64,
}

impl fmt::Debug for UploadObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadObserver").finish_non_exhaustive()
    }
}

impl UploadObserver {
    /// An observer for `operation` that delivers to `sink`.
    #[must_use]
    pub fn new(operation: OperationId, sink: Box<dyn UploadEventSink>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(State {
                sink,
                operation,
                sequence: 0,
                progress: UploadProgress::default(),
                started: false,
                finished: false,
                file_size: 0,
                file_base: 0,
            })),
        }
    }

    /// Sets the totals and sends `Started`. Later calls change nothing.
    pub(crate) fn start(&self, files_total: u64, bytes_total: u64) {
        self.with(|state| {
            if state.started {
                return;
            }
            state.progress.files_total = files_total;
            state.progress.bytes_total = bytes_total;
            state.started = true;
            state.send(UploadEventKind::Started);
        });
    }

    /// Reports the start of a revision file.
    pub(crate) fn file_started(&self, path: &RelativePath, size: u64) {
        self.with(|state| {
            state.file_size = size;
            state.send(UploadEventKind::FileStarted {
                path: path.clone(),
                size,
            });
        });
    }

    /// Reports the bytes of the running file read so far.
    pub(crate) fn file_bytes(&self, path: &RelativePath, bytes_done: u64) {
        self.with(|state| {
            state.progress.bytes_done = state.file_base + bytes_done;
            state.send(UploadEventKind::FileBytes {
                path: path.clone(),
                bytes_done,
            });
        });
    }

    /// Reports a completed revision file.
    pub(crate) fn file_finished(&self, path: &RelativePath) {
        self.with(|state| {
            state.progress.files_done += 1;
            state.file_base += state.file_size;
            state.file_size = 0;
            state.progress.bytes_done = state.file_base;
            state.send(UploadEventKind::FileFinished { path: path.clone() });
        });
    }

    /// Ends the operation with the outcome of `result`: `Finished`, `Cancelled` or `Failed`. Only the first end counts.
    pub fn end<T>(&self, result: &Result<T, Error>) {
        match result {
            Ok(_) => self.finished(),
            Err(error) if error.kind() == ErrorKind::Cancelled => self.cancelled(),
            Err(error) => self.failed(error.kind()),
        }
    }

    fn finished(&self) {
        self.terminal(UploadEventKind::Finished);
    }

    /// Ends the operation as cancelled.
    fn cancelled(&self) {
        self.terminal(UploadEventKind::Cancelled);
    }

    /// Ends the operation with a failure of this category.
    fn failed(&self, kind: ErrorKind) {
        self.terminal(UploadEventKind::Failed { kind });
    }

    fn terminal(&self, kind: UploadEventKind) {
        self.with(|state| {
            if !state.started {
                // The order holds even when the operation fails before its totals are known.
                state.started = true;
                state.send(UploadEventKind::Started);
            }
            state.send(kind);
            state.finished = true;
        });
    }

    fn with(&self, action: impl FnOnce(&mut State)) {
        let mut guard: MutexGuard<'_, State> = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        action(&mut guard);
    }
}

impl State {
    fn send(&mut self, kind: UploadEventKind) {
        if self.finished {
            return;
        }
        let event = UploadEvent {
            schema_version: UPLOAD_EVENT_SCHEMA_VERSION,
            operation: self.operation,
            sequence: self.sequence,
            kind,
            progress: self.progress,
        };
        self.sequence += 1;
        self.sink.emit(event);
    }
}

/// Reads the content of one revision file and reports every block that was read.
pub(crate) struct CountingReader {
    inner: Box<dyn Read + Send>,
    observer: UploadObserver,
    path: RelativePath,
    read: u64,
}

impl CountingReader {
    pub(crate) fn new(
        inner: Box<dyn Read + Send>,
        observer: UploadObserver,
        path: RelativePath,
    ) -> Self {
        Self {
            inner,
            observer,
            path,
            read: 0,
        }
    }
}

impl Read for CountingReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buf)?;
        if count > 0 {
            self.read += count as u64;
            self.observer.file_bytes(&self.path, self.read);
        }
        Ok(count)
    }
}
