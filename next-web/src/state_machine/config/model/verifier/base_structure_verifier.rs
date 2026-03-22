use std::{fmt::Debug, marker::PhantomData};

use crate::state_machine::config::model::{
    state_data::StateData, verifier::state_machine_model_verifier::StateMachineModelVerifier,
};

pub struct BaseStructureVerifier<S, E>(PhantomData<(S, E)>);

impl<S, E> StateMachineModelVerifier<S, E> for BaseStructureVerifier<S, E>
where
    S: Send + Sync,
    S: Debug + Clone,
    E: Send + Sync,
    E: Clone,
{
    fn verify(
        &self,
        model: &dyn crate::state_machine::config::model::state_machine_model::StateMachineModel<
            S,
            E,
        >,
    ) -> Result<(), next_web_core::error::BoxError> {
        // verify that we have transitions
        if model.get_transitions_data().transitions().is_empty() {
            return Err("Must have at least one transition".into());
        }

        // Build the state tree using user-provided state IDs
        let mut tree: Tree<StateData<S, E>> = Tree::new();
        for state_data in model.get_states_data().state_data() {
            let id: ObjectId = format!("{:?}", state_data.state()); // Convert S to String as ObjectId
            let parent: Option<ObjectId> = state_data.parent().as_ref().map(|s| format!("{:?}", s)); // Convert S to String
            tree.add(state_data.clone(), id, parent);
        }

        // Get the root node of the constructed tree
        let root_node = tree
            .get_root()
            .ok_or_else(|| "State tree is empty, cannot verify")?;

        // Verify initial state for the top level (children of the root)
        // This simulates the Java logic: find a node where node.getData() == null (which was the pseudo-root)
        // and check its children for initial states. In our case, the root itself contains data,
        // but its children are the top-level states. We check the root's children.
        let mut initial_state_found = false;
        for &child_id in &root_node.children {
            if let Some(child_node) = tree.nodes.get(child_id.0) {
                if child_node.data.is_initial() {
                    initial_state_found = true;
                    break; // Found at least one initial state in top level
                }
            }
        }

        if !initial_state_found {
            let mut trace = vec![format!("Top-level states: [")];
            for &child_id in &root_node.children {
                if let Some(child_node) = tree.nodes.get(child_id.0) {
                    trace.push(format!("\"{}\", ", child_node.user_id));
                }
            }
            if let Some(last) = trace.last_mut() {
                if last.ends_with(", ") {
                    last.truncate(last.len() - 2);
                }
            }
            trace.push("]".to_string());
            trace.push("Initial state not set for top level.".to_string());

            return Err(trace.join(" ").into()); // Combine trace into a single error message
        }

        // Note: The original Java logic seems to imply checking *groups* of states under pseudo-null nodes.
        // Our current implementation checks the immediate children of the actual root.
        // If the requirement is to handle multiple *separate* root groups, the tree building logic
        // or the verification logic needs adjustment to identify these groups.

        Ok(())
    }
}

use std::collections::{HashMap, VecDeque};

/// Represents a unique identifier for a node within the Tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NodeId(usize);

/// The core tree structure.
pub struct Tree<T> {
    /// Stores all nodes. The index into this vector serves as the NodeId.
    nodes: Vec<TreeNode<T>>,
    /// Maps user-provided IDs to internal NodeIds for quick lookup.
    id_to_node_map: HashMap<ObjectId, NodeId>,
    /// Stores pending additions until they can be properly linked.
    not_mapped: Vec<DataWrap<T>>,
    /// Optional reference to the root node's internal ID.
    root_internal_id: Option<NodeId>,
}

// Assuming 'Object' in Java context translates to a generic hashable type in Rust.
// Using Box<dyn std::any::Any + Send + Sync> is complex for keys. A simpler approach
// is to let users provide a concrete, hashable type for their ID system.
// Here, we define a placeholder type ObjectId. Users should replace this with
// their specific ID type (e.g., String, i32, etc.) or a generic parameter constrained by Hash + Eq.
type ObjectId = String; // Example: Change this to fit your specific ID type (e.g., i32, String, Uuid, etc.)

/// Represents a single node in the tree.
struct TreeNode<T> {
    /// The data stored in the node.
    data: T,
    /// IDs of child nodes.
    children: Vec<NodeId>,
    /// ID of the parent node. `None` if it's the root.
    parent: Option<NodeId>,
    /// The user-provided ID associated with this node.
    user_id: ObjectId,
}

impl<T> Tree<T> {
    /// Creates a new, empty tree. Initializes with a dummy root node at index 0.
    pub fn new() -> Self {
        Tree {
            nodes: Vec::new(),
            id_to_node_map: HashMap::new(),
            not_mapped: Vec::new(),
            root_internal_id: None,
        }
    }

    /// Gets a reference to the root node, if it exists.
    pub fn get_root(&self) -> Option<&TreeNode<T>> {
        self.root_internal_id.map(|id| &self.nodes[id.0])
    }

    /// Gets a mutable reference to the root node, if it exists.
    pub fn get_root_mut(&mut self) -> Option<&mut TreeNode<T>> {
        self.root_internal_id.map(|id| &mut self.nodes[id.0])
    }

    /// Finds a node by its user-provided ID.
    pub fn find_node_by_id(&self, id: &ObjectId) -> Option<&TreeNode<T>> {
        self.id_to_node_map
            .get(id)
            .map(|&node_id| &self.nodes[node_id.0])
    }

    /// Finds a mutable reference to a node by its user-provided ID.
    pub fn find_node_by_id_mut(&mut self, id: &ObjectId) -> Option<&mut TreeNode<T>> {
        if let Some(&node_id) = self.id_to_node_map.get(id) {
            Some(&mut self.nodes[node_id.0])
        } else {
            None
        }
    }

    /// Adds a new node to the tree.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to store in the node.
    /// * `id` - The unique user-provided ID for the node.
    /// * `parent_id` - The user-provided ID of the parent node. Use `None` if adding the root.
    pub fn add(&mut self, data: T, id: ObjectId, parent_id: Option<ObjectId>) {
        // Check if the ID already exists
        if self.id_to_node_map.contains_key(&id) {
            // Or handle error appropriately, e.g., return Result<(), E>
            eprintln!("Warning: ID {:?} already exists in the tree.", id);
            return;
        }

        self.not_mapped.push(DataWrap {
            data,
            id,
            parent: parent_id,
        });
        self.try_mapping();
    }

    /// Attempts to link pending nodes (`not_mapped`) to the main tree structure.
    fn try_mapping(&mut self) {
        loop {
            let mut processed_count = 0;

            // Iterate backwards to allow safe removal during iteration
            let mut i = 0;
            while i < self.not_mapped.len() {
                let wrap = &self.not_mapped[i];
                let parent_node_id_opt = wrap
                    .parent
                    .as_ref()
                    .and_then(|p_id| self.id_to_node_map.get(p_id).copied());

                let id = self.not_mapped[i].id.clone();
                let parent_node_id = match wrap.parent.as_ref() {
                    None => {
                        // It's intended to be the root
                        if self.root_internal_id.is_none() {
                            // Create the root node
                            let new_node_id = NodeId(self.nodes.len());
                            let new_node = TreeNode {
                                data: self.not_mapped.remove(i).data,
                                children: Vec::new(),
                                parent: None,
                                user_id: id.clone(),
                            };
                            self.nodes.push(new_node);
                            self.id_to_node_map.insert(id, new_node_id);
                            self.root_internal_id = Some(new_node_id);
                            processed_count += 1;
                            continue; // Do not increment i, check the new element at index i
                        } else {
                            i += 1;
                            continue; // Root already exists, skip this item for now
                        }
                    }
                    Some(_parent_user_id) => {
                        match parent_node_id_opt {
                            Some(id) => id,
                            None => {
                                i += 1;
                                continue;
                            } // Parent not found yet, skip
                        }
                    }
                };

                // Parent exists, try to add child
                // Need to get parent's children list to add new child
                // Borrow checker issue arises here if we try to mutably borrow nodes
                // while also borrowing it immutably inside the loop condition.
                // Solution: Get the data outside the scope needing mutable access.

                let wrap_data;
                let wrap_id;
                // Remove the item we are processing
                if i < self.not_mapped.len() {
                    let removed_wrap = self.not_mapped.remove(i); // Removes element at i, shifts others down
                    wrap_data = removed_wrap.data;
                    wrap_id = removed_wrap.id;
                } else {
                    break; // Should not happen if loop logic is correct
                }

                // Now find or create the parent's child slot safely
                // The parent node must exist because parent_node_id_opt matched Some
                // We need to add the new child to the parent's children list.

                // Create the new node first
                let new_child_internal_id = NodeId(self.nodes.len());
                let new_node = TreeNode {
                    data: wrap_data,
                    children: Vec::new(),
                    parent: Some(parent_node_id),
                    user_id: wrap_id.clone(),
                };
                self.nodes.push(new_node);
                self.id_to_node_map.insert(wrap_id, new_child_internal_id);

                // Update the parent's children list
                if let Some(parent_node) = self.nodes.get_mut(parent_node_id.0) {
                    parent_node.children.push(new_child_internal_id);
                } else {
                    // This should ideally not happen if parent_node_id is valid
                    panic!(
                        "Parent node with internal ID {:?} not found during linking.",
                        parent_node_id
                    );
                }

                processed_count += 1;
                // Do not increment i, as remove shifted elements
            }

            if processed_count == 0 {
                // No more items could be processed in this pass
                break;
            }
            // Continue loop to see if newly added parents allow more children to be linked
        }
    }

    /// Provides a breadth-first iterator over the tree starting from the root.
    pub fn bfs_iter(&self) -> BfsIter<T> {
        BfsIter::new(self)
    }
}

/// Helper struct for pending additions.
struct DataWrap<T> {
    data: T,
    id: ObjectId,
    parent: Option<ObjectId>, // Changed to Option<ObjectId> to represent potential root
}

// --- Iterator Implementation (Optional but useful) ---

pub struct BfsIter<'a, T> {
    tree: &'a Tree<T>,
    queue: VecDeque<NodeId>,
}

impl<'a, T> BfsIter<'a, T> {
    fn new(tree: &'a Tree<T>) -> Self {
        let mut queue = VecDeque::new();
        if let Some(root_id) = tree.root_internal_id {
            queue.push_back(root_id);
        }
        BfsIter { tree, queue }
    }
}

impl<'a, T> Iterator for BfsIter<'a, T> {
    type Item = &'a TreeNode<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front().map(|node_id| {
            if let Some(node) = self.tree.nodes.get(node_id.0) {
                // Add children to the queue for later processing
                for &child_id in &node.children {
                    self.queue.push_back(child_id);
                }
                node
            } else {
                // This case should not occur if NodeId is always valid
                panic!("Invalid NodeId encountered during BFS iteration.");
            }
        })
    }
}

impl<S, E> Default for BaseStructureVerifier<S, E> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
