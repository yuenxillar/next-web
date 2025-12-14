use crate::core::poi::poifs::filesystem::poifs_writer_event::POIFSWriterEvent;

/// Interface for POIFS writer listeners
pub trait POIFSWriterListener: Send + Sync {
    /// Process a POIFSWriterEvent that this listener had registered for
    ///
    /// # Arguments
    /// * `event` - The POIFSWriterEvent
    fn process_poifs_writer_event(&self, event: &POIFSWriterEvent);
}
