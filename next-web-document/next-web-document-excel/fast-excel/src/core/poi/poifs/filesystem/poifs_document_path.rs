use std::fmt;

/// Represents a POIFS document path
pub struct POIFSDocumentPath {
    components: Vec<String>,
    hashcode: u64, // lazy-computed hashCode
}

impl POIFSDocumentPath {
    /// Simple constructor for the path of a document that is in the root of the POIFSFileSystem.
    /// The constructor that takes an array of Strings can also be used to create such a
    /// POIFSDocumentPath by passing it a null or empty String array
    pub fn new() -> Self {
        POIFSDocumentPath {
            components: Vec::new(),
            hashcode: 0,
        }
    }

    /// Constructor for the path of a document that is not in the root of the POIFSFileSystem
    ///
    /// # Arguments
    /// * `components` - The Strings making up the path to a document.
    ///   The Strings must be ordered as they appear in the directory hierarchy of the document.
    ///   The first string must be the name of a directory in the root of the POIFSFileSystem, and
    ///   every Nth (for N > 1) string thereafter must be the name of a directory in the directory
    ///   identified by the (N-1)th string.
    ///   If the components parameter is null or has zero length, the POIFSDocumentPath is appropriate
    ///   for a document that is in the root of a POIFSFileSystem
    ///
    /// # Errors
    /// * Returns error if any of the elements in the components parameter are null or have zero length
    pub fn from_components(components: &[String]) -> Result<Self, String> {
        Self::from_path_and_components(None, components)
    }

    /// Constructor that adds additional subdirectories to an existing path
    ///
    /// # Arguments
    /// * `path` - The existing path
    /// * `components` - The additional subdirectory names to be added
    ///
    /// # Errors
    /// * Returns error if any of the Strings in components is null or zero length
    pub fn from_path_and_components(
        path: Option<&POIFSDocumentPath>,
        components: &[String],
    ) -> Result<Self, String> {
        let s1 = match path {
            Some(p) => p.components.clone(),
            None => Vec::new(),
        };

        // Check for null or empty strings
        for (i, component) in components.iter().enumerate() {
            if component.is_empty() {
                return Err(format!("Component {} cannot be empty", i));
            }
        }

        let mut all_components = s1;
        all_components.extend_from_slice(components);

        Ok(POIFSDocumentPath {
            components: all_components,
            hashcode: 0,
        })
    }

    /// Get the number of components
    ///
    /// # Returns
    /// * Number of components
    pub fn length(&self) -> usize {
        self.components.len()
    }

    /// Get the specified component
    ///
    /// # Arguments
    /// * `n` - Which component (0 ... length() - 1)
    ///
    /// # Returns
    /// * The nth component
    ///
    /// # Panics
    /// * Panics if n < 0 or n >= length()
    pub fn get_component(&self, n: usize) -> &str {
        &self.components[n]
    }

    /// Get the path's parent or `None` if this path is the root path
    ///
    /// # Returns
    /// * Path of parent, or `None` if this path is the root path
    pub fn get_parent(&self) -> Option<POIFSDocumentPath> {
        if self.components.is_empty() {
            None
        } else {
            let parent_components = self.components[..self.components.len() - 1].to_vec();
            Some(POIFSDocumentPath {
                components: parent_components,
                hashcode: 0,
            })
        }
    }

    /// Get the last name in the document path's name sequence.
    /// If the document path's name sequence is empty, then the empty string is returned.
    ///
    /// # Returns
    /// * The last name in the document path's name sequence, or empty string if this is the root path
    pub fn get_name(&self) -> &str {
        if self.components.is_empty() {
            ""
        } else {
            &self.components[self.components.len() - 1]
        }
    }
}

impl Default for POIFSDocumentPath {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for POIFSDocumentPath {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components
    }
}

impl Eq for POIFSDocumentPath {}

impl fmt::Display for POIFSDocumentPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/")?;
        for (i, component) in self.components.iter().enumerate() {
            if i > 0 {
                write!(f, "/")?;
            }
            write!(f, "{}", component)?;
        }
        Ok(())
    }
}
