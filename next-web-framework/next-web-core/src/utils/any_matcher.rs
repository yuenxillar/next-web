use std::{cell::UnsafeCell, cmp::min, fmt, mem, ops::Range};

#[derive(Clone, Debug)]
pub struct AnyMatcher<T> {
    pub(crate) root: Node<T>,
}

impl<T> Default for AnyMatcher<T> {
    fn default() -> Self {
        Self {
            root: Node::default(),
        }
    }
}

impl<T> AnyMatcher<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, route: impl Into<String>, value: T) -> Result<(), InsertError> {
        self.root.insert(route.into(), value)
    }

    pub fn at(&self, path: &str) -> Result<Match<&T>, MatchError> {
        match self.root.at(path.as_bytes()) {
            Ok(value) => Ok(Match {
                // Safety: We only expose `&mut T` through `&mut self`
                value: unsafe { &*value.get() },
            }),
            Err(e) => Err(e),
        }
    }

    pub fn at_mut(&mut self, path: &str) -> Result<Match<&mut T>, MatchError> {
        match self.root.at(path.as_bytes()) {
            Ok(value) => Ok(Match {
                // Safety: We have `&mut self`
                value: unsafe { &mut *value.get() },
            }),
            Err(e) => Err(e),
        }
    }
}

/// A successful match consisting of the registered value
/// and URL parameters, returned by [`Router::at`](Router::at).
#[derive(Debug)]
pub struct Match<V> {
    /// The value stored under the matched node.
    pub value: V,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MatchError {
    /// No matching route was found.
    NotFound,
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Matching route not found")
    }
}

impl std::error::Error for MatchError {}

pub struct Node<T> {
    // This node's prefix.
    pub(crate) prefix: Prefix,
    // The priority of this node.
    //
    // Nodes with more children are higher priority and searched first.
    pub(crate) priority: u32,
    // Whether this node contains a wildcard child.
    pub(crate) wild_child: bool,
    // The first character of any static children, for fast linear search.
    pub(crate) indices: Vec<u8>,
    // The type of this node.
    pub(crate) node_type: NodeType,
    pub(crate) children: Vec<Self>,
    // The value stored at this node.
    //
    // See `Node::at` for why an `UnsafeCell` is necessary.
    value: Option<UnsafeCell<T>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub enum NodeType {
    /// The root path.
    Root,
    /// A static prefix, e.g. `/foo`.
    Static,
    /// A catch-all parameter, e.g. `/**`.
    CatchAll,
    /// A single catch-all parameter, e.g. `/*`.
    CatchSingle,
    /// A single catch-all parameter, e.g. `/*.js`.
    CatchSingleExtension,
}

impl<T> Node<T> {
    // Insert a route into the tree.
    pub fn insert(&mut self, route: String, val: T) -> Result<(), InsertError> {
        if route.is_empty() || *"/" != route[0..1] {
            return Err(InsertError::InvalidFormat);
        }

        if route.eq("/**") {
            return Err(InsertError::InvalidFormat);
        }

        let prefix = Prefix::new(route.clone().into_bytes());

        self.priority += 1;

        let mut remaining = prefix.inner.clone();
        let mut current = self;

        'walk: loop {
            // Find the common prefix between the route and the current node.
            let len = min(remaining.len(), current.prefix.len());
            let common_prefix = (0..len)
                .find(|&i| remaining[i] != current.prefix.inner[i])
                .unwrap_or(len);

            // If this node has a longer prefix than we need, we have to fork and extract the
            // common prefix into a shared parent.
            if current.prefix.len() > common_prefix {
                // Move the non-matching suffix into a child node.
                let suffix = current.prefix.slice_off(common_prefix);
                let child = Node {
                    prefix: suffix,
                    value: current.value.take(),
                    indices: current.indices.clone(),
                    wild_child: current.wild_child,
                    children: mem::take(&mut current.children),
                    priority: current.priority - 1,
                    node_type: NodeType::Static,
                };

                // The current node now only holds the common prefix.
                current.children = vec![child];
                current.indices = vec![current.prefix.inner[common_prefix]];
                current.prefix = current.prefix.slice_until(common_prefix);
                current.wild_child = false;
                continue;
            }

            if remaining.len() == common_prefix {
                // This node must not already contain a value.
                if current.value.is_some() {
                    return Err(InsertError::conflict(&route, &prefix, current));
                }

                // Insert the value.
                current.value = Some(UnsafeCell::new(val));
                return Ok(());
            }

            // Otherwise, the route has a remaining non-matching suffix.
            //
            // We have to search deeper.
            remaining = remaining[common_prefix..].to_vec();
            let next = remaining[0];

            // After matching against a wildcard the next character is always `/`.
            //
            // Continue searching in the child node if it already exists.
            if current.node_type == NodeType::CatchSingleExtension && current.children.len() == 1 {
                debug_assert_eq!(next, b'/');
                current = &mut current.children[0];
                current.priority += 1;
                continue 'walk;
            }

            // Find a child node that matches the next character in the route.
            for mut i in 0..current.indices.len() {
                if next == current.indices[i] {
                    if matches!(next, b'*') {
                        continue;
                    }

                    // Continue searching in the child.
                    i = current.update_child_priority(i);
                    current = &mut current.children[i];
                    continue 'walk;
                }
            }

            // We couldn't find a matching child.
            //
            // If we're not inserting a wildcard we have to create a child.
            if (!matches!(next, b'*')) && current.node_type != NodeType::CatchAll {
                current.indices.push(next);
                let mut child = current.add_child(Node::default());
                child = current.update_child_priority(child);

                // Insert into the newly created node.
                let _last = current.children[child].insert_route(prefix.clone(), val)?;
                return Ok(());
            }

            // We're trying to insert a wildcard.
            //
            // If this node already has a wildcard child, we have to make sure it matches.
            if current.wild_child {
                // Wildcards are always the last child.
                current = current.children.last_mut().unwrap();
                current.priority += 1;

                // Make sure the route parameter matches.
                if let Some(wildcard) = remaining.get(..current.prefix.len()) {
                    if *wildcard != *current.prefix.inner {
                        return Err(InsertError::conflict(&route.clone(), &prefix, current));
                    }
                }

                // Catch-all routes cannot have children.
                if current.node_type == NodeType::CatchAll {
                    return Err(InsertError::conflict(&route, &prefix, current));
                }

                // Continue with the wildcard node.
                continue 'walk;
            }

            // Otherwise, create a new node for the wildcard and insert the route.
            let _last = current.insert_route(prefix, val)?;
            return Ok(());
        }
    }

    /// Removes a route from the tree, returning the value if the route already existed.
    ///
    /// The provided path should be the same as the one used to insert the route, including
    /// wildcards.
    pub fn remove(&mut self, route: String) -> Option<T> {
        let route = Prefix::new(route.into_bytes());
        let remaining = route.inner;

        // Check if we are removing the root node.
        if remaining == self.prefix.inner {
            let value = self.value.take().map(UnsafeCell::into_inner);

            // If the root node has no children, we can reset it.
            if self.children.is_empty() {
                *self = Node::default();
            }

            return value;
        }

        let mut current = self;
        'walk: loop {
            // The path is longer than this node's prefix, search deeper.
            if remaining.len() > current.prefix.len() {
                let (prefix, rest) = remaining.split_at(current.prefix.len());

                // The prefix matches.
                if prefix == current.prefix.inner {
                    let first = rest[0];

                    // If there is a single child node, we can continue searching in the child.
                    if current.children.len() == 1 {
                        // The route matches, remove the node.
                        if current.children[0].prefix.inner == remaining {
                            return current.remove_child(0);
                        }

                        // Otherwise, continue searching.
                        current = &mut current.children[0];
                        continue 'walk;
                    }

                    // Find a child node that matches the next character in the route.
                    if let Some(i) = current.indices.iter().position(|&c| c == first) {
                        // The route matches, remove the node.
                        if current.children[i].prefix.inner == remaining {
                            return current.remove_child(i);
                        }

                        // Otherwise, continue searching.
                        current = &mut current.children[i];
                        continue 'walk;
                    }

                    // If the node has a matching wildcard child, continue searching in the child.
                    if current.wild_child
                        && remaining.first().zip(remaining.get(2)) == Some((&b'{', &b'}'))
                    {
                        // The route matches, remove the node.
                        if current.children.last_mut().unwrap().prefix.inner == remaining {
                            return current.remove_child(current.children.len() - 1);
                        }

                        current = current.children.last_mut().unwrap();
                        continue 'walk;
                    }
                }
            }

            // Could not find a match.
            return None;
        }
    }

    /// Remove the child node at the given index, if the route parameters match.
    fn remove_child(&mut self, i: usize) -> Option<T> {
        // If the node does not have any children, we can remove it completely.
        let value = if self.children[i].children.is_empty() {
            // Removing a single child with no indices.
            if self.children.len() == 1 && self.indices.is_empty() {
                self.wild_child = false;
                self.children.remove(0).value.take()
            } else {
                // Remove the child node.
                let child = self.children.remove(i);

                match child.node_type {
                    // Remove the index if we removed a static prefix.
                    NodeType::Static => {
                        self.indices.remove(i);
                    }
                    // Otherwise, we removed a wildcard.
                    _ => self.wild_child = false,
                }

                child.value
            }
        }
        // Otherwise, remove the value but preserve the node.
        else {
            self.children[i].value.take()
        };

        value.map(UnsafeCell::into_inner)
    }

    // Adds a child to this node, keeping wildcards at the end.
    fn add_child(&mut self, child: Node<T>) -> usize {
        let len = self.children.len();

        if self.wild_child && len > 0 {
            self.children.insert(len - 1, child);
            len - 1
        } else {
            self.children.push(child);
            len
        }
    }

    // Increments priority of the given child node, reordering the children if necessary.
    //
    // Returns the new index of the node.
    fn update_child_priority(&mut self, i: usize) -> usize {
        self.children[i].priority += 1;
        let priority = self.children[i].priority;

        // Move the node to the front as necessary.
        let mut updated = i;
        while updated > 0 && self.children[updated - 1].priority < priority {
            self.children.swap(updated - 1, updated);
            updated -= 1;
        }

        // Update the position of the indices to match.
        if updated != i {
            self.indices[updated..=i].rotate_right(1);
        }

        updated
    }

    // Insert a route at this node.
    fn insert_route(&mut self, mut prefix: Prefix, val: T) -> Result<&mut Node<T>, InsertError> {
        let mut current = self;

        loop {
            // Search for a wildcard segment.
            let wildcard = match find_wildcard(prefix.inner.as_ref())? {
                Some(wildcard) => wildcard,
                // There is no wildcard, simply insert into the current node.
                None => {
                    current.value = Some(UnsafeCell::new(val));
                    current.prefix = prefix.to_owned();
                    return Ok(current);
                }
            };

            println!("range: {:?}", wildcard);

            // Insering a catch-all route.
            if &prefix.inner[wildcard.clone()] == "**".as_bytes() {
                // Add the prefix before the wildcard into the current node.
                if wildcard.start > 0 {
                    current.prefix = prefix.slice_until(wildcard.start);
                    prefix = prefix.slice_off(wildcard.start);
                }

                // Add the catch-all as a child node.
                let child = Self {
                    prefix: prefix.to_owned(),
                    node_type: NodeType::CatchAll,
                    value: Some(UnsafeCell::new(val)),
                    priority: 1,
                    ..Self::default()
                };

                let i = current.add_child(child);
                current.wild_child = true;
                return Ok(&mut current.children[i]);
            }

            // Otherwise, we're inserting a regular route parameter.
            assert_eq!(prefix.inner[wildcard.clone()][0], b'*');

            // /api/*
            // /app/*.js

            // Add the prefix before the wildcard into the current node.
            if wildcard.start > 0 {
                current.prefix = prefix.slice_until(wildcard.start);
                prefix = prefix.slice_off(wildcard.start);
            }

            // Add the parameter as a child node.
            let child = Self {
                node_type: if prefix
                    .inner
                    .get(wildcard.start..wildcard.start + 2)
                    .unwrap_or("".as_bytes())
                    == "*.".as_bytes()
                {
                    NodeType::CatchSingleExtension
                } else {
                    NodeType::CatchSingle
                },
                prefix: prefix.slice_until(wildcard.len()),
                ..Self::default()
            };

            println!("node: {:?}", child.prefix);

            let child = current.add_child(child);
            current.wild_child = true;
            current = &mut current.children[child];
            current.priority += 1;

            // If the route doesn't end in the wildcard, we have to insert the suffix as a child.
            if wildcard.len() < prefix.inner.len() {
                prefix = prefix.slice_off(wildcard.len());
                let child = Self {
                    priority: 1,
                    ..Self::default()
                };

                let child = current.add_child(child);
                current = &mut current.children[child];
                continue;
            }

            // Finally, insert the value.
            current.value = Some(UnsafeCell::new(val));
            return Ok(current);
        }
    }

    pub fn at<'node>(&'node self, full_path: &[u8]) -> Result<&'node UnsafeCell<T>, MatchError> {
        let mut current = self;
        let mut path = full_path;
        let mut backtracking = false;
        let mut skipped_nodes: Vec<Skipped<'_, '_, T>> = Vec::new();

        'walk: loop {
            // Reached the end of the search
            if path.len() <= current.prefix.len() {
                if *path == *current.prefix.inner {
                    if let Some(ref value) = current.value {
                        return Ok(value);
                    }
                }

                if let Some(skipped) = skipped_nodes.pop() {
                    path = skipped.path;
                    current = skipped.node;
                    backtracking = true;
                    continue 'walk;
                }
                println!("1");
                return Err(MatchError::NotFound);
            }

            // Split path at current prefix
            let (prefix, rest) = path.split_at(current.prefix.len());

            if *prefix != *current.prefix.inner {
                if let Some(skipped) = skipped_nodes.pop() {
                    path = skipped.path;
                    current = skipped.node;
                    backtracking = true;
                    continue 'walk;
                }
                println!(
                    "{}, {}, len: {}",
                    String::from_utf8_lossy(prefix),
                    String::from_utf8_lossy(&current.prefix.inner),
                    current.prefix.len()
                );

                return Err(MatchError::NotFound);
            }

            // path = rest;

            println!("path: {}", String::from_utf8_lossy(path));

            if !backtracking {
                if let Some(&next_char) = path.get(0) {
                    if let Some(index) = current.indices.iter().position(|&c| c == next_char) {
                        if current.wild_child {
                            skipped_nodes.push(Skipped {
                                path,
                                node: current,
                            });
                        }
                        current = &current.children[index];
                        continue 'walk;
                    }
                }
            }

            if !current.wild_child {
                if let Some(skipped) = skipped_nodes.pop() {
                    path = skipped.path;

                    current = skipped.node;
                    backtracking = true;
                    continue 'walk;
                }
                return Err(MatchError::NotFound);
            }

            current = current.children.last().ok_or(MatchError::NotFound)?;
            match current.node_type {
                NodeType::CatchSingle => {
                    let segment_end = path.iter().position(|&c| c == b'/').unwrap_or(path.len());
                    if segment_end == 0 {
                        if let Some(skipped) = skipped_nodes.pop() {
                            path = skipped.path;
                            current = skipped.node;
                            backtracking = true;
                            continue 'walk;
                        }
                        return Err(MatchError::NotFound);
                    }

                    // For CatchSingle, we must match exactly one segment
                    if segment_end == path.len() {
                        if let Some(ref value) = current.value {
                            return Ok(value);
                        }
                    } else if path[segment_end] == b'/' {
                        if let Some(child) = current.children.first() {
                            path = &path[segment_end + 1..];
                            current = child;
                            backtracking = false;
                            continue 'walk;
                        }
                    }

                    if let Some(skipped) = skipped_nodes.pop() {
                        path = skipped.path;
                        current = skipped.node;
                        backtracking = true;
                        continue 'walk;
                    }
                    return Err(MatchError::NotFound);
                }
                NodeType::CatchAll => {
                    // For CatchAll, match all remaining path segments
                    if let Some(ref value) = current.value {
                        return Ok(value);
                    }
                    return Err(MatchError::NotFound);
                }
                _ => unreachable!(),
            }
        }
    }
}

struct Skipped<'n, 'p, T> {
    // The node that was skipped.
    node: &'n Node<T>,
    /// The path at the time we skipped this node.
    path: &'p [u8],
}

#[rustfmt::skip]
#[macro_export]
macro_rules! backtracker {
    ($skipped_nodes:ident, $path:ident, $current:ident, $backtracking:ident, $walk:lifetime) => {
        macro_rules! try_backtrack {
            () => {
                // Try backtracking to any matching wildcard nodes that we skipped while
                // traversing the tree.
                while let Some(skipped) = $skipped_nodes.pop() {
                    if skipped.path.ends_with($path) {
                        // Restore the search state.
                        $path = skipped.path;
                        $current = &skipped.node;
                        $backtracking = true;
                        continue $walk;
                    }
                }
            };
        }
    };
}

// Searches for a wildcard segment and checks the path for invalid characters.
fn find_wildcard(path: &Vec<u8>) -> Result<Option<Range<usize>>, InsertError> {
    for (start, &c) in path.iter().enumerate() {
        // 跳过不是未转义的 '*' 字符
        if c != b'*' {
            continue;
        }

        let end: usize = path.len();

        // 检查是否 '**' or *.js
        if let Some(&next_char) = path.get(start + 1) {
            if next_char == b'*' {
                if start + 2 != path.len() {
                    return Err(InsertError::InvalidParamSegment);
                }
            } else if next_char == b'.' {
                for i in path[start + 2..].iter() {
                    if i == &b'/' {
                        return Err(InsertError::InvalidParamSegment);
                    }
                }
            }
        }

        return Ok(Some(start..end));
    }

    Ok(None)
}

#[derive(Clone)]
pub struct Prefix {
    // The raw prefix route.
    pub(crate) inner: Vec<u8>,
    index: Option<(usize, Vec<u8>)>,
}

impl Default for Prefix {
    fn default() -> Self {
        Self {
            inner: vec![47],
            index: Default::default(),
        }
    }
}

impl fmt::Debug for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Prefix")
            .field("inner", &String::from_utf8_lossy(&self.inner))
            .finish()
    }
}

impl Prefix {
    /// /test
    /// /test/*
    /// /test/**
    /// /test/*.js
    pub fn new(mut inner: Vec<u8>) -> Self {
        let mut i = 0;
        while let Some(&c) = inner.get(i) {
            if c == b'*' {
                inner.remove(i);
            }
            i += 1;
        }

        Self { inner, index: None }
    }

    pub fn slice_until(&self, end: usize) -> Self {
        Self {
            inner: self.inner[..end].to_owned(),
            index: None,
        }
    }

    /// Slices the route with `start..`.
    pub fn slice_off(&self, start: usize) -> Self {
        Self {
            inner: self.inner[start..].to_owned(),
            index: None,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

/// Represents errors that can occur when inserting a new route.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum InsertError {
    /// Attempted to insert a path that conflicts with an existing route.
    Conflict {
        /// The existing route that the insertion is conflicting with.
        with: String,
    },
    /// Route should start with "/"
    InvalidFormat,
    /// Parameters must be registered with a valid name and matching braces.
    ///
    /// Note you can use `{{` or `}}` to escape literal brackets.
    InvalidParam,
    InvalidParamSegment,
    /// Catch-all parameters are only allowed at the end of a path.
    InvalidCatchAll,
}

impl fmt::Display for InsertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { with } => {
                write!(
                    f,
                    "Insertion failed due to conflict with previously registered route: {}",
                    with
                )
            }
            Self::InvalidParamSegment => write!(f, "Invalid parameter segment"),
            Self::InvalidFormat => write!(f, "Route should start with \"/\""),
            Self::InvalidParam => write!(f, "Parameters must be registered with a valid name"),
            Self::InvalidCatchAll => write!(
                f,
                "Catch-all parameters are only allowed at the end of a route"
            ),
        }
    }
}

impl std::error::Error for InsertError {}

impl InsertError {
    /// Returns an error for a route conflict with the given node.
    ///
    /// This method attempts to find the full conflicting route.
    pub(crate) fn conflict<T>(route: &str, prefix: &Prefix, current: &Node<T>) -> Self {
        let mut route = route.as_bytes().to_vec();

        // The route is conflicting with the current node.
        if &prefix.inner == &current.prefix.inner {
            return InsertError::Conflict {
                with: String::from_utf8(route).unwrap_or(String::from("invalid utf8")),
            };
        }

        // Remove the non-matching suffix from the route.
        route.truncate(route.len() - prefix.len());

        // Add the conflicting prefix.
        if !route.ends_with(&current.prefix.inner) {
            route.extend(&current.prefix.inner);
        }

        // Add the prefixes of any conflicting children.
        let mut child = current.children.first();
        while let Some(node) = child {
            route.extend(&node.prefix.inner);
            child = node.children.first();
        }

        // Denormalize any route parameters.
        let mut last = current;
        while let Some(node) = last.children.first() {
            last = node;
        }

        // Return the conflicting route.
        InsertError::Conflict {
            with: String::from_utf8(route).unwrap_or(String::from("invalid utf8")),
        }
    }
}

impl<T> Clone for Node<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let value = self.value.as_ref().map(|value| {
            // Safety: We only expose `&mut T` through `&mut self`.
            let value = unsafe { &*value.get() };
            UnsafeCell::new(value.clone())
        });

        Self {
            value,
            prefix: self.prefix.clone(),
            wild_child: self.wild_child,
            node_type: self.node_type.clone(),
            indices: self.indices.clone(),
            children: self.children.clone(),
            priority: self.priority,
        }
    }
}

/// Safety: We expose `value` per Rust's usual borrowing rules, so we can just
/// delegate these traits.
unsafe impl<T: Send> Send for Node<T> {}
unsafe impl<T: Sync> Sync for Node<T> {}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self {
            prefix: Prefix::default(),
            wild_child: true,
            node_type: NodeType::Root,
            indices: Vec::new(),
            children: Vec::new(),
            value: None,
            priority: 0,
        }
    }
}

impl<T> fmt::Debug for Node<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: We only expose `&mut T` through `&mut self`.
        let value = unsafe { self.value.as_ref().map(|x| &*x.get()) };

        let mut f = f.debug_struct("Node");
        f.field("value", &value)
            .field("node_type", &self.node_type)
            .field("prefix", &self.prefix)
            .field("children", &self.children);

        // Extra information for debugging purposes.
        #[cfg(test)]
        {
            let indices = self
                .indices
                .iter()
                .map(|&x| char::from_u32(x as _))
                .collect::<Vec<_>>();
            f.field("indices", &indices);
        }

        f.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::any_matcher::AnyMatcher;

    #[test]
    fn test_find_wildcard() {
        let mut matcher = AnyMatcher::new();
        matcher.insert("/app/*", String::from("6666")).unwrap();

        // println!("{:?}", matcher);

        matcher.insert("/666/**", String::from("7777")).unwrap();

        println!("{:?}", matcher);

        println!("value: {:?}", matcher.at("/666/test/666"));
        println!("value: {:?}", matcher.at("/666/test"));
    }
}
