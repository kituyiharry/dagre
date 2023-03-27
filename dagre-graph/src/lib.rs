////////////////////////////////////////////////////////////////////////////
//                                                                        //
//                       Trait based Graph implementation                 //
//                                                                        //
//                          ////////////////////////////////////////////////
//                          //
//  @author: Harry K        //
//  @date    25th Mar 2023  //
//                          //
//////////////////////////////

use std::{rc::{Rc, Weak}, hash::Hash, cell::RefCell, io};
use std::fmt::{Debug, Display};
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::io::Write;
use std::io::BufWriter;
use std::borrow::Cow;

// Quick reference counted container with interior mutability
type RcRef<T> = Rc<RefCell<T>>;

// Quick shared container with interior mutability
type WkRef<T> = Weak<RefCell<T>>;

// make_rc_ref boilerplate that creates a reference counted container that wraps the supplied value
#[inline(always)]
fn make_owned<T>(t: T) -> RcRef<T> {
    Rc::new(RefCell::new(t))
}

// make_shared boilerplate that creates a shared container structure that wraps the supplied value
#[inline(always)]
fn make_shared<T>(t: &Rc<RefCell<T>>) -> WkRef<T> {
    Rc::downgrade(t)
}

// NodeLike describes some property of your node in the graph here, We need to uniquely
// index it and also store the data. Implement it for what your graph node would be
//
// If you want mutable access maybe have a separate RefCell wrapping your data so that you can .get
// on it when you need it from the graph later on
pub trait NodeLike {
    // Associated struct which is the data in the node - needs the following keys
    type Unique: Eq + Hash + Ord + Debug;
    // A way to access this data - for consistency we may not want to mutate it after we add it!
    // the graph will internally add Rc and RefCell to it
    fn unique(&self) -> Self::Unique;
    // Label for your Node data
    fn label(&self) -> String;
}

// Node
// why 'a:
// references: https://users.rust-lang.org/t/why-this-impl-type-lifetime-may-not-live-long-enough/67855/2
// Internal way of holding your supplied node as a Graph
pub struct Node<'a, I> where I: Eq + Hash + Ord + Debug {
    // Your data wrapped for the graph
    pub data: Box<dyn NodeLike<Unique=I> + 'a>,
}

////////////////////////////////////////////////////////
//                                                    //
//  Trait Implementations for Node used in the graph  //
//                                                    //
////////////////////////////////////////////////////////

// Hash
impl<I: Eq + Hash + Ord + Debug> Hash for Node<'_, I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.unique().hash(state)
    }
}

// PartialEq
impl<I: Eq + Hash + Ord + Debug> PartialEq for Node<'_,I> {
    fn eq(&self, other: &Self) -> bool {
        self.data.unique().eq(&other.data.unique())
    }
}

// Eq
impl<I: Eq + Hash + Ord + Debug> Eq for Node<'_,I> {
    fn assert_receiver_is_total_eq(&self) {}
}

// PartialOrd
impl<I: Hash + Eq + Ord + Debug> PartialOrd for Node<'_,I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data.unique().partial_cmp(&other.data.unique())
    }
}

// Ord
impl<I: Hash + Eq + Ord + Debug> Ord for Node<'_,I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.unique().cmp(&other.data.unique())
    }
}

// Debug
impl<I: Hash + Eq + Ord + Debug> Debug for Node<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<< {:?} >>", self.data.unique())
    }
}

// Display
impl<I: Hash + Eq + Ord + Debug> Display for Node<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "< {} >", self.data.label())
    }
}

// Impl Node
impl<'a, I: Hash + Eq + Ord + Debug> Node<'a, I> {
    // Create a new graph node from your trait implementation
    pub fn create(subject: impl NodeLike<Unique=I> + 'a) -> Self {
        Node {
            data: Box::new(subject)
        }
    }
}


// EdgeSet is a unique collection T nodes in an edge for a given node in the graph
pub type EdgeSet<'a,I> = Vec<WkRef<Node<'a,I>>>;

// // Incoming and Outgoing edges
#[derive(Debug, Default)]
pub struct Edges<'a,I: Hash + Ord + Eq + Debug>(EdgeSet<'a,I>, EdgeSet<'a,I>, DagreRingLog<'a, 20>);

// Type aliases for weak references to refcells of nodes
type WeakNode<'a, I>  = Weak<RefCell<Node<'a,I>>>;
// Type aliases for strong references to refcells of nodes
type StrongNode<'a,I> = Rc<RefCell<Node<'a,I>>>;

// Type aliases for weak and strong nodes
impl<'a, I: Ord + Hash + Eq + Debug> Edges<'a,I> {

    // New placeholder for incoming and outgoing edges
    #[inline(always)]
    pub fn new() -> Self {
       Edges(EdgeSet::new(), EdgeSet::new(), DagreRingLog::default())
    }

    // Get the incoming edges
    #[inline(always)]
    pub fn incoming(&self) -> &EdgeSet<'a,I> {
        &self.0
    }

    // Get the incoming edges
    #[inline(always)]
    pub fn mut_incoming(&mut self) -> &mut EdgeSet<'a,I> {
        &mut self.0
    }

    // Get the incoming edges
    #[inline(always)]
    pub fn logs(&self) -> &DagreRingLog<'a, 20> {
        &self.2
    }

    // Get the incoming edges
    #[inline(always)]
    pub fn mut_logs(&mut self) -> &mut DagreRingLog<'a, 20> {
        &mut self.2
    }

    // Get the outgoing edges
    #[inline(always)]
    pub fn outgoing(&self) -> &EdgeSet<'a,I> {
        &self.1
    }

    // Get the outgoing edges
    #[inline(always)]
    pub fn mut_outgoing(&mut self) -> &mut EdgeSet<'a,I> {
        &mut self.1
    }

    // Add a node val to the incoming edge a given node
    #[inline(always)]
    pub fn add_to_incoming(&mut self, val: &WeakNode<'a,I>) {
        self.0.push(Weak::clone(val))
    }

    // Add a node val to the outgoing edge a given node
    #[inline(always)]
    pub fn add_to_outgoing(&mut self, val: &WeakNode<'a,I>) {
        self.1.push(Weak::clone(val))
    }

}

// Type alias for our Graph based on BTreeMap
pub type DaggerMapGraph<'a,I> = BTreeMap<StrongNode<'a,I>, Edges<'a,I>>;

// MakeGraph interface for defining some graph operations
pub trait MakeGraph<'a, I: Ord + Debug + Display + Hash> {
    // node adds a new member into the graph definition
    fn node(&mut self, val: impl NodeLike<Unique=I> + 'a) -> WeakNode<'a,I>;
    // edge adds a connection from one node to another if available - or does nothing otherwise
    fn edge(&mut self, valfrom: &WeakNode<'a,I>, valto: &WeakNode<'a,I>);
    // Find by value (useful when reference isn't available)
    fn find(&self, val: impl NodeLike<Unique=I> + 'a) -> Option<(WeakNode<'a,I>, &Edges<'a, I>)>;
    // Find by reference (useful if Weak pointer available)
    fn get_by(&self, val: &WeakNode<'a, I>) -> Option<&Edges<'a, I>>;
    // Remove a node
    fn unlink(&mut self, node: &WeakNode<'a,I>);
    // TODO: subgraph (vertex induced and shallow?)
    //fn induce(&self, nodes: Vec<StrongNode<'a, ...>>) -> Self;
}

impl<'a, I: Ord + Debug + Display + Hash> MakeGraph<'a, I> for DaggerMapGraph<'a, I> {

    // TODO: "tag" the weak references returned with something unique to this graph! so not just
    // any weak ref can be added if it doesn't exist
    fn node(&mut self, data: impl NodeLike<Unique = I> + 'a) -> WeakNode<'a, I> {
        let lab = data.label();
        let new_node = make_owned(Node::create(data));
        // Check if already exists
        if let Some((k,_)) = self.get_key_value(&new_node) {
            return Rc::downgrade(k)
        }
        let ret_node = make_shared(&new_node);
        let edges = self.entry(new_node).or_insert_with(Edges::new);
        edges.mut_logs().write(DagreEvent::Add(lab.into()));
        ret_node
    }

    // TODO: make less reliant on weakrefs and directly use impl NodeLike
    fn edge(&mut self, origin: &WeakNode<'a,I>, destination: &WeakNode<'a,I>) {
        // valfrom ---> valto
            if let (Some(frompresence), Some(topresence)) = (origin.upgrade(), destination.upgrade()) {
                if let Some(edgefrom) = self.get_mut(&frompresence) {
                    let lab = topresence.borrow().data.label();
                    // add destination to origin
                    edgefrom.add_to_outgoing(destination);
                    edgefrom.mut_logs().write(DagreEvent::To(lab.into()));
                }
                if let Some(edgeto) = self.get_mut(&topresence) {
                    let lab = frompresence.borrow().data.label();
                    // add origin to incoming edge of destination
                    edgeto.add_to_incoming(origin);
                    edgeto.mut_logs().write(DagreEvent::From(lab.into()));
                }
                // TODO: Check if succeeded
            }
        // Can't find anything so don't do anything
    }

    fn find(&self, val: impl NodeLike<Unique=I> + 'a) -> Option<(WeakNode<'a,I>, &Edges<'a, I>)> {
        let fnode = make_owned(Node::create(val));
        if let Some((k, v)) = self.get_key_value(&fnode) {
            return Some((make_shared(k), v))
        }
        None
    }

    fn get_by(&self, val: &WeakNode<'a, I>) -> Option<&Edges<'a, I>> {
        if let Some(presence) =  val.upgrade() {
            if let Some(v) = self.get(&presence) {
                return Some(v)
            }
        }
        None
    }

    // TODO: Clear weak refs after unlinking a weak - hint: use 
    fn unlink(&mut self, node: &WeakNode<'a,I>) {
        if let Some(presence) =  node.upgrade() {
            if let Some(mut edges) = self.remove(&presence) {
                edges.mut_logs().write(
                    DagreEvent::Remove(
                        presence.borrow().data.label().into()
                    )
                );
            }
        }
    }

}

///////////////////////
//  Graph Event Log  //
///////////////////////

// DagreEvent 
pub enum DagreEvent<'a> {
    Add(Cow<'a, str>),
    From(Cow<'a, str>),
    To(Cow<'a, str>),
    Remove(Cow<'a, str>)
}

impl<'a> ToString for DagreEvent<'a> {
    fn to_string(&self) -> String {
        match self {
            DagreEvent::Add(addition) => {
                String::from(format!("[+]   {}\n", addition.as_ref()))
            },
            DagreEvent::From(from) => {
                String::from(format!("{} -> *\n",from.as_ref()))
            },
            DagreEvent::To(to) => {
                String::from(format!("*  -> {}\n",to.as_ref()))
            },
            DagreEvent::Remove(subtracted) => {
                String::from(format!("[-]   {}\n", subtracted.as_ref()))
            },
        }
    }
}

pub trait EventLogWriter {
    fn write(&mut self, event: DagreEvent);
}

#[derive(Debug)]
pub struct DagreRingLog<'a, const BUFSIZE: usize> {
    pub log_buf: VecDeque<Cow<'a, str>>
}

impl<'a, const BUFSIZE:usize> Default for DagreRingLog<'a, BUFSIZE> {
    fn default() -> Self {
        Self {
            log_buf: VecDeque::with_capacity(BUFSIZE)
        }
    }
}

impl<'a, const BUFSIZE: usize> DagreRingLog<'a, BUFSIZE> {
    pub fn new() -> Self {
        Self {
            log_buf: VecDeque::with_capacity(BUFSIZE)
        }
    }

    pub fn dumps(&self, writeloc: impl Write) -> io::Result<()> {
        let mut bufw = BufWriter::new(writeloc);
        self.log_buf.iter().rev().for_each(|log| {
            if let Ok(_num) = bufw.write(log.as_ref().as_bytes()) {};
        });
        bufw.flush()
    }
}

impl<const BUFSIZE: usize> EventLogWriter for DagreRingLog<'_, BUFSIZE> {
    fn write(&mut self, event: DagreEvent) {
        self.log_buf.push_front(event.to_string().into())
    }
}

pub struct NopEventLogWriter();

impl EventLogWriter for NopEventLogWriter {
    fn write(&mut self, _: DagreEvent) {}
}
