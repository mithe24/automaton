use automaton::{Automaton, Symbol};
use graphviz_rust::printer::{DotPrinter, PrinterContext};

fn main() {
    let mut automaton = Automaton::new("q0");

    let q1 = automaton.add_state("final");

    automaton.add_transition(
        automaton.initial(),
        Symbol::char('a'),
        q1.clone(),
    );

    automaton.add_transition(
        automaton.initial(),
        Symbol::char('b'),
        q1.clone(),
    );

    automaton.mark_state_accepting(q1);

    let graph = automaton.to_graph();

    println!("{}", graph.print(&mut PrinterContext::default()));
}
