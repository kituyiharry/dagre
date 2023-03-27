use dagre_graph::{DaggerMapGraph, MakeGraph, NodeLike};

#[derive(Debug)]
pub struct UWrap(usize);

impl NodeLike for UWrap {

    type Unique = usize;

    fn unique(&self) -> Self::Unique {
        self.0
    }

    fn label(&self) -> String {
        self.0.to_string().into()
    }

}

fn main() {
    let mut graph = DaggerMapGraph::new();
    let n = graph.node(UWrap(10));
    let x = graph.node(UWrap(40));
    let j = graph.node(UWrap(20));
    let q = graph.node(UWrap(30));
    let p = graph.node(UWrap(40)); // Already inserted so
    // p and x should be the same thing!!
    graph.edge(&n, &j);
    graph.edge(&p, &j);
    graph.edge(&q, &j);
    graph.edge(&n, &q);
    graph.edge(&n, &p);
    graph.edge(&n, &n); //Self reference
    //
    println!("{:#?}", p.upgrade());

    if let Some(edges) = graph.get_by(&p) {
        println!("p");
        if let Ok(()) = edges.logs().dumps(std::io::stdout()) {}
    }

    if let Some(edges) = graph.get_by(&x) {
        println!("x");
        if let Ok(()) = edges.logs().dumps(std::io::stdout()) {}
    }

    println!("{}", graph.len());
    println!("======================");
    graph.iter().for_each(|(k,v)| {
        println!("{:?} :: ({}, {})", k, v.incoming().len(), v.outgoing().len())
    });
    println!("======================");
    graph.unlink(&n); //Self reference
    //graph.unlink(&j); //Self reference
    println!("{:#?}", n.upgrade());
    graph.iter().for_each(|(k,v)| {
        println!("{:?} :: ({}, {})", k, v.incoming().len(), v.outgoing().len())
    });
    graph.unlink(&n);
}
