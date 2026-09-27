use automaton::{Automaton, Symbol};
use graphviz_rust::printer::{DotPrinter, PrinterContext};

fn main() {
    let mut automaton = Automaton::new("start");
    let q1 = automaton.add_state("final").unwrap();
    let q2 = automaton.add_state("intermediate").unwrap();
    automaton.mark_state_accepting(q1.clone()).unwrap();

    automaton
        .add_transition(automaton.initial(), Symbol::char('a'), q1.clone())
        .unwrap();
    automaton
        .add_transition(automaton.initial(), Symbol::char('b'), q1.clone())
        .unwrap();

    automaton
        .add_transition(automaton.initial(), Symbol::char('a'), q2.clone())
        .unwrap();
    automaton
        .add_transition(q2.clone(), Symbol::epsilon(), q2.clone())
        .unwrap();
    automaton
        .add_transition(q2.clone(), Symbol::char('b'), q1.clone())
        .unwrap();

    let graph = automaton.to_graph();
    println!("{}", graph.print(&mut PrinterContext::default()));
}
