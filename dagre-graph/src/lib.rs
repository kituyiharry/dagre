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
    fn label(&self) -> Box<[u8]>;
}

// Node
// why 'a:
// references: https://users.rust-lang.org/t/why-this-impl-type-lifetime-may-not-live-long-enough/67855/2
// Internal way of holding your supplied node as a Graph
pub struct DagreNode<'a, I> where I: Eq + Hash + Ord + Debug {
    // Your data wrapped for the graph
    pub data: Box<dyn NodeLike<Unique=I> + 'a>,
    // TODO: Some other tracking metadata
}

impl<I: Hash + Eq + Debug + Ord> PartialOrd for Box<dyn NodeLike<Unique=I>> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.unique().partial_cmp(&other.unique())
    }
}

impl<I: Hash + Eq + Debug + Ord> PartialEq for Box<dyn NodeLike<Unique=I>> {
    fn eq(&self, other: &Self) -> bool {
        self.unique().eq(&other.unique())
    }
}

impl<I: Hash + Eq + Debug + Ord> Eq for Box<dyn NodeLike<Unique=I>> {
    fn assert_receiver_is_total_eq(&self) {}
}

////////////////////////////////////////////////////////
//                                                    //
//  Trait Implementations for Node used in the graph  //
//                                                    //
////////////////////////////////////////////////////////

// Hash
impl<I: Eq + Hash + Ord + Debug> Hash for DagreNode<'_, I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.unique().hash(state)
    }
}

// PartialEq
impl<I: Eq + Hash + Ord + Debug> PartialEq for DagreNode<'_,I> {
    fn eq(&self, other: &Self) -> bool {
        self.data.unique().eq(&other.data.unique())
    }
}

// Eq
impl<I: Eq + Hash + Ord + Debug> Eq for DagreNode<'_,I> {
    fn assert_receiver_is_total_eq(&self) {}
}

// PartialOrd
impl<I: Hash + Eq + Ord + Debug> PartialOrd for DagreNode<'_,I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data.unique().partial_cmp(&other.data.unique())
    }
}

// Ord
impl<I: Hash + Eq + Ord + Debug> Ord for DagreNode<'_,I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.unique().cmp(&other.data.unique())
    }
}

// Debug
impl<I: Hash + Eq + Ord + Debug> Debug for DagreNode<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<< {:?} >>", self.data.unique())
    }
}

// Display
impl<I: Hash + Eq + Ord + Debug> Display for DagreNode<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let clstr = self.data.label();
            let selfstr = std::str::from_utf8_unchecked(clstr.as_ref());
            write!(f, "< {} >",  selfstr)
        }
    }
}

// Impl Node
impl<'a, I: Hash + Eq + Ord + Debug> DagreNode<'a, I> {
    // Create a new graph node from your trait implementation
    pub fn create(subject: impl NodeLike<Unique=I> + 'a) -> Self {
        DagreNode {
            data: Box::new(subject)
        }
    }
}


// EdgeSet is a unique collection T nodes in an edge for a given node in the graph
pub type EdgeSet<'a,I> = Vec<WkRef<DagreNode<'a,I>>>;

// // Incoming and Outgoing edges
#[derive(Debug, Default)]
pub struct Edges<'a,I: Hash + Ord + Eq + Debug>(EdgeSet<'a,I>, EdgeSet<'a,I>, DagreRingLog<'a, 20>);

// Type aliases for weak references to refcells of nodes
type WeakNode<'a, I>  = Weak<RefCell<DagreNode<'a,I>>>;
// Type aliases for strong references to refcells of nodes
type StrongNode<'a,I> = Rc<RefCell<DagreNode<'a,I>>>;

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

    // Remove a node val 
    pub fn invalidate_from(mut self, graph: &mut impl DagreProtocol<'a, I>, labelremoved: Box<[u8]>) {
        // ---- Remove from the outgoing of incoming nodes
        self.mut_incoming().iter_mut().for_each(|inc| {
            if let Some(infiltered) = graph.get_by_mut(inc) {
                infiltered.mut_logs().write(DagreEvent::Remove(Cow::Borrowed(labelremoved.as_ref())));
                infiltered.mut_outgoing().retain(|o| {
                    o.strong_count() != 0
                })
            }
        });
        // ---- Remove from the incoming of outgoing nodes
        self.mut_outgoing().iter_mut().for_each(|out| {
            if let Some(outfiltered) = graph.get_by_mut(out) {
                outfiltered.mut_logs().write(DagreEvent::Remove(Cow::Borrowed(labelremoved.as_ref())));
                outfiltered.mut_incoming().retain(|i| {
                    i.strong_count() != 0
                })
            }
        });
    }

}

// Type alias for our Graph based on BTreeMap
pub type DaggerMapGraph<'a,I> = BTreeMap<StrongNode<'a,I>, Edges<'a,I>>;

// MakeGraph interface for defining some graph operations
pub trait DagreProtocol<'a, I: Ord + Debug + Hash> {
    // node adds a new member into the graph definition
    fn node(&mut self, val: impl NodeLike<Unique=I> + 'a) -> WeakNode<'a,I>;
    // edge adds a connection from one node to another if available - or does nothing otherwise
    fn unidirectional(&mut self, valfrom: &WeakNode<'a,I>, valto: &WeakNode<'a,I>);
    // edge adds a connection from one node to another if available - or does nothing otherwise
    fn bidirectional(&mut self, valfrom: &WeakNode<'a,I>, valto: &WeakNode<'a,I>);
    // Find by value (useful when reference isn't available)
    fn find(&self, val: impl NodeLike<Unique=I> + 'a) -> Option<(WeakNode<'a,I>, &Edges<'a, I>)>;
    // Find by reference (useful if Weak pointer available)
    fn get_by(&self, val: &WeakNode<'a, I>) -> Option<&Edges<'a, I>>;
    // Remove a node
    fn evict(&mut self, node: &WeakNode<'a,I>);
    // get edges mutably
    fn get_by_mut(&mut self, val: &WeakNode<'a, I>) -> Option<&mut Edges<'a, I>>;
    // edge deletion
    fn unlink(&mut self, from: &WeakNode<'a,I>, to: &WeakNode<'a,I>);
    //fn induce(&self, nodes: Vec<StrongNode<'a, ...>>) -> Self;
}

impl<'a, I: Ord + Debug + Display + Hash> DagreProtocol<'a, I> for DaggerMapGraph<'a, I> {

    // TODO: "tag" the weak references returned with something unique to this graph! so not just
    // any weak ref can be added if it doesn't exist
    fn node(&mut self, data: impl NodeLike<Unique = I> + 'a) -> WeakNode<'a, I> {
        let lab = data.label();
        let new_node = make_owned(DagreNode::create(data));
        // Check if already exists
        if let Some((k,_)) = self.get_key_value(&new_node) {
            return Rc::downgrade(k)
        }
        let ret_node = make_shared(&new_node);
        let edges = self.entry(new_node).or_insert_with(Edges::new);
        edges.mut_logs().write(DagreEvent::Add(Cow::Borrowed(lab.as_ref())));
        ret_node
    }

    fn unidirectional(&mut self, origin: &WeakNode<'a,I>, destination: &WeakNode<'a,I>) {
        if let (Some(frompresence), Some(topresence)) = (origin.upgrade(), destination.upgrade()) {
            if let Some(edgefrom) = self.get_mut(&frompresence) {
                let lab = topresence.borrow().data.label();
                // add destination to origin
                edgefrom.add_to_outgoing(destination);
                edgefrom.mut_logs().write(DagreEvent::To(Cow::Borrowed(lab.as_ref())));
            }
            if let Some(edgeto) = self.get_mut(&topresence) {
                let lab = frompresence.borrow().data.label();
                // add origin to incoming edge of destination
                edgeto.add_to_incoming(origin);
                edgeto.mut_logs().write(DagreEvent::From(Cow::Borrowed(lab.as_ref())));
            }
            // TODO: Check if succeeded
        }
    }

    fn bidirectional(&mut self, origin: &WeakNode<'a,I>, destination: &WeakNode<'a,I>) {
        if let (Some(frompresence), Some(topresence)) = (origin.upgrade(), destination.upgrade()) {
            if let Some(edgefrom) = self.get_mut(&frompresence) {
                let lab = topresence.borrow().data.label();
                let flab = frompresence.borrow().data.label();
                // add destination to origin
                edgefrom.add_to_outgoing(destination);
                edgefrom.add_to_incoming(destination);
                edgefrom.mut_logs().write(DagreEvent::To(Cow::Borrowed(lab.as_ref())));
                edgefrom.mut_logs().write(DagreEvent::From(Cow::Borrowed(flab.as_ref())));
            }
            if let Some(edgeto) = self.get_mut(&topresence) {
                let lab = frompresence.borrow().data.label();
                let tlab = frompresence.borrow().data.label();
                // add origin to incoming edge of destination
                edgeto.add_to_incoming(origin);
                edgeto.add_to_outgoing(origin);
                edgeto.mut_logs().write(DagreEvent::From(Cow::Borrowed(lab.as_ref())));
                edgeto.mut_logs().write(DagreEvent::To(Cow::Borrowed(tlab.as_ref())));
            }
            // TODO: Check if succeeded
        }
    }

    fn find(&self, val: impl NodeLike<Unique=I> + 'a) -> Option<(WeakNode<'a,I>, &Edges<'a, I>)> {
        let fnode = make_owned(DagreNode::create(val));
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

    fn get_by_mut(&mut self, val: &WeakNode<'a, I>) -> Option<&mut Edges<'a, I>> {
        if let Some(presence) =  val.upgrade() {
            if let Some(v) = self.get_mut(&presence) {
                return Some(v)
            }
        }
        None
    }

    // TODO: Clear weak refs after unlinking a weak - hint: use 
    fn evict(&mut self, node: &WeakNode<'a,I>) {
        if let Some(presence) =  node.upgrade() {
            if let Some(edges) = self.remove(&presence) {
                let label = presence.borrow().data.label();
                // Invalidate weak references to this node
                drop(presence);
                edges.invalidate_from(self, label);
            }
        }
    }

    // TODO: Clear weak refs after unlinking a weak - hint: use 
    fn unlink(&mut self, from: &WeakNode<'a,I>, to: &WeakNode<'a,I>) {
        if let (Some(fromp), Some(top)) =  (from.upgrade(), to.upgrade()) {
            if let Some(edges) = self.get_by_mut(&from) {
                if let Some(pos) = edges.mut_outgoing().iter().position(|o| {
                    if let Some(up) = o.upgrade() {
                        return top.borrow().eq(&up.borrow())
                    }
                    false
                }) {
                    edges.mut_outgoing().remove(pos);
                }
            }
            if let Some(edges) = self.get_by_mut(&to) {
                if let Some(pos) = edges.mut_incoming().iter().position(|o| {
                    if let Some(up) = o.upgrade() {
                        return fromp.borrow().eq(&up.borrow())
                    }
                    false
                }) {
                    edges.mut_incoming().remove(pos);
                }
            }
        }
    }

}

///////////////////////
//  Graph Event Log  //
///////////////////////

// DagreEvent 
pub enum DagreEvent<'a> {
    Add(Cow<'a, [u8]>),
    From(Cow<'a, [u8]>),
    To(Cow<'a, [u8]>),
    Remove(Cow<'a, [u8]>),
    UnlinkInc(Cow<'a, [u8]>),
    UnlinkOut(Cow<'a, [u8]>),
}

impl<'a> ToString for DagreEvent<'a> {
    fn to_string(&self) -> String {
        unsafe {
            match self {
                DagreEvent::Add(addition) => {
                    String::from(format!("[+]   {}\n", std::str::from_utf8_unchecked(addition.as_ref())))
                },
                DagreEvent::From(from) => {
                    String::from(format!("{} -> *\n", std::str::from_utf8_unchecked(from.as_ref())))
                },
                DagreEvent::To(to) => {
                    String::from(format!("*  -> {}\n", std::str::from_utf8_unchecked(to.as_ref())))
                },
                DagreEvent::Remove(subtracted) => {
                    String::from(format!("[-]   {}\n", std::str::from_utf8_unchecked(subtracted.as_ref())))
                },
                DagreEvent::UnlinkInc(other) => {
                    String::from(format!("* -/-> {}\n", std::str::from_utf8_unchecked(other.as_ref())))
                },
                DagreEvent::UnlinkOut(other) => {
                    String::from(format!("{} -/-> *\n", std::str::from_utf8_unchecked(other.as_ref())))
                },
            }
        }
    }
}

pub trait EventLogWriter {
    fn write(&mut self, event: DagreEvent);
}

#[derive(Debug)]
pub struct DagreRingLog<'a, const BUFSIZE: usize> {
    pub log_buf: VecDeque<Cow<'a, [u8]>>
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

    pub fn dumps(&self, writer: impl Write) -> io::Result<()> {
        let mut bufw = BufWriter::new(writer);
        self.log_buf.iter().rev().for_each(|log| {
            if let Ok(_num) = bufw.write(log.as_ref()) {};
        });
        bufw.flush()
    }
}

impl<const BUFSIZE: usize> EventLogWriter for DagreRingLog<'_, BUFSIZE> {
    fn write(&mut self, event: DagreEvent) {
        self.log_buf.push_front(
           Cow::Owned(
                (event.to_string()).as_bytes().to_vec()
            )
        )
    }
}

pub struct NopEventLogWriter();



impl EventLogWriter for NopEventLogWriter {
    fn write(&mut self, _: DagreEvent) {}
}

#[cfg(test)]
mod tests {

    use crate::NodeLike;

    ////////////////////////////////
    //  New graph implementation  //
    ////////////////////////////////
    
    use super::{DagreProtocol, DaggerMapGraph};

    pub struct TestNode(usize);

    impl NodeLike for TestNode {
        type Unique = usize;

        fn unique(&self) -> Self::Unique {
            self.0
        }

        fn label(&self) -> Box<[u8]> {
            self.0.to_string().into_boxed_str().into_boxed_bytes()
        }
    }

    #[test]
    fn graph_node_added() {
        let mut graph = DaggerMapGraph::new();
        graph.node(TestNode(20));
        graph.node(TestNode(30));
        assert_eq!(graph.len(), 2);
    }

    #[test]
    fn graph_edge_added() {
        let mut graph = DaggerMapGraph::new();
        let i = graph.node(TestNode(20));
        let j = graph.node(TestNode(30));
        graph.unidirectional(&i,&j);
        graph.unidirectional(&j,&i);
        let b = graph.get_by(&i).unwrap();
        let q = graph.get_by(&j).unwrap();
        assert_eq!(b.incoming().len(), 1);
        assert_eq!(b.outgoing().len(), 1);
        assert_eq!(q.incoming().len(), 1);
        assert_eq!(q.outgoing().len(), 1);
    }

    #[test]
    fn graph_node_removed() {
        let mut graph = DaggerMapGraph::new();
        let i = graph.node(TestNode(20));
        let j = graph.node(TestNode(30));
        graph.unidirectional(&i,&j);
        graph.unidirectional(&j,&i);
        graph.evict(&i);
        let b = graph.get_by(&i);
        let q = graph.get_by(&j).unwrap();
        assert!(b.is_none());
        assert_eq!(graph.len(), 1);
        assert_eq!(q.incoming().len(), 0);
    }

    #[test]
    fn graph_edge_removed() {
        let mut graph = DaggerMapGraph::new();
        let i = graph.node(TestNode(20));
        let j = graph.node(TestNode(30));
        graph.unidirectional(&i,&j);
        graph.unidirectional(&j,&i);
        graph.unlink(&i,&j);
        graph.unlink(&j,&i);
        let b = graph.get_by(&i).unwrap();
        let q = graph.get_by(&j).unwrap();
        assert_eq!(graph.len(), 2);
        assert_eq!(q.incoming().len(), 0);
        assert_eq!(q.outgoing().len(), 0);
        assert_eq!(b.incoming().len(), 0);
        assert_eq!(b.outgoing().len(), 0);
    }

    #[test]
    fn graph_directionality() {
        let mut graph = DaggerMapGraph::new();
        let i = graph.node(TestNode(20));
        let j = graph.node(TestNode(30));
        // ---
        let k = graph.node(TestNode(40));
        let l = graph.node(TestNode(50));
        // Introduce scope to handle double mut with immut
        {
            graph.unidirectional(&i,&j);
            let b = graph.get_by(&i).unwrap();
            let q = graph.get_by(&j).unwrap();
            assert_eq!(q.incoming().len(), 1);
            assert_eq!(q.outgoing().len(), 0);
            assert_eq!(b.incoming().len(), 0);
            assert_eq!(b.outgoing().len(), 1);
        }
        {
            graph.bidirectional(&k, &l);
            let m = graph.get_by(&k).unwrap();
            let n = graph.get_by(&l).unwrap();
            assert_eq!(m.incoming().len(), 1);
            assert_eq!(m.outgoing().len(), 1);
            assert_eq!(n.incoming().len(), 1);
            assert_eq!(n.outgoing().len(), 1);
        }
        // everything should be there still
        assert_eq!(graph.len(), 4);
    }


}
