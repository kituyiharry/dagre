use std::{time::{Duration}, thread::sleep};

use dagger_graph::{DaggerGraph, MakeGraph, NodeLike};

#[derive(Debug)]
pub struct UWrap(usize);

impl<'a> NodeLike for &'a UWrap {

    type Unique = &'a usize;

    fn unique(&self) -> Self::Unique {
        &(self.0)
    }

}

fn main() {
    sleep(Duration::from_secs(5));
    let mut graph = DaggerGraph::new();
    let (n,_) = graph.node(&UWrap(10));
    let (j,_) = graph.node(&UWrap(20));
    let (q,_) = graph.node(&UWrap(30));
    let (n2,_) = graph.node(&UWrap(40));
    graph.edge(&n, &j);
    graph.edge(&n2, &j);
    graph.edge(&q, &j);
    graph.edge(&q, &n);
    graph.edge(&n, &n); //Self reference
    println!("{}", graph.len());
    println!("======================");
    println!("{:#?}", graph);
    println!("======================");
    graph.unlink(&n); //Self reference
    println!("{:#?}", graph);
    println!("{:#?}", n.upgrade())
}
