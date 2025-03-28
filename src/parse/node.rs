//! These definitions are lifted directly from Starling, and allow seamless interaction with it.
//! Changes to anything in here *should* be breaking on Starling's end, but watch to make sure!
//!
//! See https://github.com/arctic-hen7/starling:src/node.rs.

use orgish::Timestamp;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use uuid::Uuid;

/// A representation of all the information about a single node in the graph.
///
/// The information returned can be regulated with [`NodeOptions`].
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Node {
    // --- Basics ---
    /// The node's unique identifier.
    pub id: Uuid,
    /// The title of this node and its parents.
    pub title: Vec<String>,
    /// The path this node came from.
    pub path: PathBuf,
    /// The tags on this node itself. There will be no duplicates here.
    pub tags: HashSet<String>,
    /// The tags on this node's parents. There will be no duplicates here.
    pub parent_tags: HashSet<String>,
    /// The ID of the parent, if there is one (this will be `None` for top-level nodes).
    pub parent_id: Option<Uuid>,

    // --- Metadata ---
    /// The metadata about the node, if requested.
    ///
    /// NOTE: We will always request the metadata, but this has to be optional for it to
    /// deserialize correctly from bincode.
    pub metadata: Option<NodeMetadata>,

    /// The body of the node, if requested. This may be arbitrarily large.
    ///
    /// If the body is not requested, this will be `None`, but it could also be `None` if the node
    /// has no body. For most uses, `None` can be treated as an empty string (though technically
    /// that is just a blank line, as opposed to the immediate start of the next node).
    pub body: Option<String>,

    /// The unique identifiers of all the *direct* children of this node. Unlike child connections,
    /// this will *not* traverse the entire tree. Each child will also have its title reported for
    /// easy reference (the full title paths will be these, appended onto the title array of this
    /// their parent node).
    ///
    /// This will only be populated if the children are requested.
    ///
    /// NOTE: We will always request the children.
    pub children: Vec<(Uuid, String)>,

    // --- Connection information ---
    /// Any valid connections this node has directly to other nodes.
    ///
    /// This will only be populated if connection information is requested.
    pub connections: HashMap<Uuid, NodeConnection>,
    /// Valid connections this node's children have to other nodes. These will be combined with
    /// each other. No information about which children different connections come from is
    /// preserved.
    ///
    /// If the requested node and one or more of its children connect to the same node, the
    /// connection will be recorded on the root only (with all the types from the children).
    ///
    /// This will only be populated if both connection and child connection information is
    /// requested.
    pub child_connections: HashMap<Uuid, NodeConnection>,
    /// Connections from other nodes *to* this specific node.
    ///
    /// This will only be populated if connection information is requested.
    pub backlinks: HashMap<Uuid, NodeConnection>,
    /// Connections from other nodes *to* any of the children of this node.
    ///
    /// This will only be populated if both connection and child connection information is
    /// requested.
    pub child_backlinks: HashMap<Uuid, NodeConnection>,
}

/// Metadata about a node. This is a simplification of the representation in a [`StarlingNode`] for
/// transmission.
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct NodeMetadata {
    /// The level of this node (0 for a root node) in the hierarchhy of the document it came from.
    /// This is essentially the number of `#`s at the start of the node in Markdown (or `*`s in
    /// Org).
    pub level: u8,
    /// The priority note on this heading, if one was present. These can contain any kind of
    /// string.
    pub priority: Option<String>,
    /// A deadline on this node, if present.
    pub deadline: Option<Timestamp>,
    /// A scheduled timestamp on this node, if present. This is typically used to indicate when an
    /// action item should be started.
    pub scheduled: Option<Timestamp>,
    /// A closed timestamp on this node, if present.
    // TODO: What are these used for??
    pub closed: Option<Timestamp>,
    /// The properties of the node. These are totally freeform.
    pub properties: HashMap<String, String>,
    /// A keyword at the start of the node, which will be one of the ones in the global config if
    /// it's present. These are used to indicate action states, like `TODO` or `NEXT`.
    pub keyword: Option<String>,
    /// Timestamps at the end of the node.
    pub timestamps: Vec<Timestamp>,
}

/// A self-contained representation of a connection with (either to or from) another node. This
/// doesn't include the ID of the other node, just because it's used in maps where that information
/// is known from the key.
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct NodeConnection {
    /// The other node's raw title.
    pub title: Vec<String>,
    /// The types of the connection (one node can connect with another multiple times, this
    /// aggregates all the different types).
    pub types: HashSet<String>,
}

/// Options that can be used to customize the information returned about a node.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct NodeOptions {
    /// Whether or not to return the body of this node (this may be arbitrarily large).
    #[serde(default)]
    pub body: bool,
    /// Whether or not to return metadata about the requested node itself, like schedule
    /// information, and properties. Particularly properties may be arbitrarily large. Note that
    /// tags will always be returned.
    #[serde(default)]
    pub metadata: bool,
    /// Whether or not to return the IDs of the direct children of this node.
    #[serde(default)]
    pub children: bool,
    /// Whether or not to return connections and backlinks for this node. This doesn't incur
    /// additional computation so much as additional locking, so it should be avoided if it isn't
    /// needed.
    #[serde(default)]
    pub connections: bool,
    /// Whether or not to return connections and backlinks in the children. These "logically"
    /// inherit upwards (e.g. if another node connects to a node a child, then it implicitly
    /// connects to the parents too). This incurs quite a bit of extra computation, so should only
    /// be used when necessary.
    ///
    /// If this is `true` and `connections` is false, this will be treated as `false`.
    #[serde(default)]
    pub child_connections: bool,
    /// The format links should be serialized to (Markdown or Org).
    pub conn_format: Format,
}

/// The format of a node (here, only used to determine which format links should be serialized to).
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Markdown,
    Org,
}
