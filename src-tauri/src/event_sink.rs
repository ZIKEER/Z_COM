use crate::models::TransportEvent;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub(crate) struct EventSink {
    pub(crate) config_directory: PathBuf,
    dispatch: Arc<dyn Fn(TransportEvent) + Send + Sync>,
}

impl EventSink {
    pub(crate) fn new(
        config_directory: PathBuf,
        dispatch: impl Fn(TransportEvent) + Send + Sync + 'static,
    ) -> Self {
        Self {
            config_directory,
            dispatch: Arc::new(dispatch),
        }
    }

    pub(crate) fn dispatch(&self, event: TransportEvent) {
        (self.dispatch)(event);
    }
}
