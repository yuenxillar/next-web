use crate::core::poi::poifs::filesystem::poifs_document_path::POIFSDocumentPath;

/// Represents a POIFS writer event
pub struct POIFSWriterEvent {
    // stream: Box<dyn DocumentOutputStream>,
    path: POIFSDocumentPath,
    document_name: String,
    limit: usize,
}

impl POIFSWriterEvent {
    /// Create a new POIFSWriterEvent
    ///
    /// # Arguments
    /// * `stream` - The DocumentOutputStream, freshly opened
    /// * `path` - The path of the document
    /// * `document_name` - The name of the document
    /// * `limit` - The limit, in bytes, that can be written to the stream
    pub fn new(
        // stream: Box<dyn DocumentOutputStream>,
        path: POIFSDocumentPath,
        document_name: String,
        limit: usize,
    ) -> Self {
        // POIFSWriterEvent {
        //     // stream,
        //     path,
        //     document_name,
        //     limit,
        // }
        todo!()
    }

    /// Get the DocumentOutputStream, freshly opened
    ///
    /// # Returns
    /// * The DocumentOutputStream
    // pub fn get_stream(&self) -> &dyn DocumentOutputStream {
    //     &*self.stream
    // }

    /// Get the document's path
    ///
    /// # Returns
    /// * The document's path
    pub fn get_path(&self) -> &POIFSDocumentPath {
        &self.path
    }

    /// Get the document's name
    ///
    /// # Returns
    /// * The document's name
    pub fn get_name(&self) -> &str {
        &self.document_name
    }

    /// Get the limit on writing, in bytes
    ///
    /// # Returns
    /// * The limit in bytes
    pub fn get_limit(&self) -> usize {
        self.limit
    }
}
