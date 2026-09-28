use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    rc::Rc,
};

use thiserror::Error;

use crate::AutomatonError::DuplicateState;

pub type StateRef = Rc<State>;

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub struct State {
    name: String,
}

impl State {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub enum Symbol {
    Epsilon,
    Symbol(char),
}

impl Into<char> for Symbol {
    fn into(self) -> char {
        match self {
            Symbol::Epsilon => 'ε',
            Symbol::Symbol(c) => c,
        }
    }
}

impl From<char> for Symbol {
    fn from(c: char) -> Self {
        Self::Symbol(c)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::Epsilon => write!(f, "ε"),
            Symbol::Symbol(c) => write!(f, "{c}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AutomatonError {
    /// A state with this name already exists in the automaton.
    #[error("state already exists: {0}")]
    DuplicateState(String),

    /// The given state does not belong to this automaton.
    #[error("state does not belong to this automaton: {0}")]
    UnknownState(StateRef),

    /// DFAs cannot contain epsilon transitions.
    #[error("DFAs cannot contain epsilon transitions")]
    EpsilonNotAllowedInDfa,

    /// A DFA already has an outgoing transition for this (state, symbol).
    #[error("DFA transition already exists for {from} / {symbol}")]
    DuplicateDfaTransition { from: StateRef, symbol: Symbol },
}

type Result<T> = std::result::Result<T, AutomatonError>;

#[derive(Debug)]
pub struct Automaton {
    states: Vec<StateRef>,
    state_set: HashSet<StateRef>,
    accepting: HashSet<StateRef>,
    symbols: HashSet<Symbol>,
    initial: StateRef,
    transitions: HashMap<StateRef, HashMap<Symbol, HashSet<StateRef>>>,
    fresh_counter: usize,
}

impl Automaton {
    pub fn new(initial_name: impl Into<String>) -> Self {
        let initial = Rc::new(State::new(initial_name));
        let mut state_set = HashSet::new();
        state_set.insert(initial.clone());
        Self {
            states: vec![initial.clone()],
            state_set,
            accepting: HashSet::new(),
            symbols: HashSet::new(),
            initial,
            transitions: HashMap::new(),
            fresh_counter: 0,
        }
    }

    pub fn initial(&self) -> StateRef {
        self.initial.clone()
    }

    pub fn states(&self) -> impl Iterator<Item = StateRef> + '_ {
        self.states.iter().cloned()
    }

    pub fn accepting(&self) -> &HashSet<StateRef> {
        &self.accepting
    }

    /// Adds a new state and returns its name.
    pub fn add_state(&mut self, name: impl Into<String>) -> Result<StateRef> {
        let name = name.into();

        let state = Rc::new(State::new(name.clone()));

        if self.state_set.contains(&state) {
            return Err(DuplicateState(name));
        }

        self.states.push(state.clone());
        self.state_set.insert(state.clone());
        Ok(state)
    }

    /// Adds a state with an automatically generated, guaranteed-unique
    /// name of the form `q0`, `q1`, ... Used by algorithms (like subset
    /// construction's trap state) that need a fresh state without caring
    /// what it's called. A user-chosen name is never rejected for
    /// "looking like" one of these; on the rare occasion of an actual
    /// collision this just tries the next counter value.
    pub fn add_fresh_state(&mut self) -> StateRef {
        loop {
            let candidate: String = format!("q{}", self.fresh_counter).into();
            self.fresh_counter += 1;

            if let Ok(state) = self.add_state(candidate) {
                return state;
            }
        }
    }

    /// The outgoing transitions of `state`, by symbol, if it has any.
    pub fn transitions_from(
        &self,
        state: &StateRef,
    ) -> Option<&HashMap<Symbol, HashSet<StateRef>>> {
        self.transitions.get(state)
    }

    pub fn alphabet(&self) -> impl Iterator<Item = Symbol> {
        self.symbols
            .iter()
            .filter(|symbol| !matches!(symbol, Symbol::Epsilon))
            .cloned()
    }

    pub fn is_accepting(&self, state: &StateRef) -> bool {
        self.accepting.contains(state)
    }

    pub fn mark_state_accepting(&mut self, state: StateRef) -> Result<()> {
        self.require_state(&state)?;
        self.accepting.insert(state.clone());
        Ok(())
    }

    /// Adds a transition between two states for the symbol provided;
    /// states have to exist.
    pub fn add_transition(
        &mut self,
        source: StateRef,
        symbol: Symbol,
        target: StateRef,
    ) -> Result<()> {
        self.require_state(&source)?;
        self.require_state(&target)?;

        self.transitions
            .entry(source.clone())
            .or_default()
            .entry(symbol.clone())
            .or_default()
            .insert(target.clone());
        self.symbols.insert(symbol.clone());
        Ok(())
    }

    pub fn all_transitions(
        &self,
    ) -> impl Iterator<Item = (StateRef, Symbol, StateRef)> + '_ {
        self.transitions.iter().flat_map(|(source, by_symbol)| {
            by_symbol.iter().flat_map(move |(symbol, targets)| {
                targets.iter().map(move |target| {
                    (source.clone(), symbol.clone(), target.clone())
                })
            })
        })
    }

    pub fn reachable_from(
        &self,
        sources: impl IntoIterator<Item = StateRef>,
        symbol: &Symbol,
    ) -> HashSet<StateRef> {
        sources
            .into_iter()
            .flat_map(|source| {
                self.transitions
                    .get(&source)
                    .and_then(|by_symbol| by_symbol.get(symbol))
                    .into_iter()
                    .flatten()
                    .cloned()
            })
            .collect()
    }

    /// Checks that `state` belongs to this automaton, without mutating
    /// anything. Exposed so callers can validate a `StateRef` up front
    /// (e.g. before batching several operations) instead of only finding
    /// out via an `Err` from a mutating call.
    pub fn require_state(&self, state: &StateRef) -> Result<()> {
        if self.state_set.contains(state) {
            Ok(())
        } else {
            Err(AutomatonError::UnknownState(state.clone()))
        }
    }

    pub fn to_graph(&self) -> graphviz_rust::dot_structures::Graph {
        render_graph(
            self.states.clone(),
            self.accepting(),
            &self.initial,
            self.all_transitions().collect(),
        )
    }
}

impl Default for Automaton {
    fn default() -> Self {
        Self::new("q0")
    }
}

#[derive(Debug)]
pub struct DFA {
    automaton: Automaton,
    transitions: HashMap<(StateRef, char), StateRef>,
    symbols: HashSet<Symbol>,
}

impl DFA {
    pub fn new(initial_name: impl Into<String>) -> Self {
        Self {
            automaton: Automaton::new(initial_name),
            transitions: HashMap::new(),
            symbols: HashSet::new(),
        }
    }

    pub fn initial(&self) -> StateRef {
        self.automaton.initial()
    }

    pub fn states(&self) -> impl Iterator<Item = StateRef> + '_ {
        self.automaton.states()
    }

    pub fn alphabet(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.automaton.alphabet()
    }

    pub fn is_accepting(&self, state: &StateRef) -> bool {
        self.automaton.is_accepting(state)
    }

    pub fn add_state(&mut self, name: impl Into<String>) -> Result<StateRef> {
        self.automaton.add_state(name)
    }

    pub fn mark_state_accepting(&mut self, state: StateRef) -> Result<()> {
        self.automaton.mark_state_accepting(state)
    }

    /// DFA transitions must be non-epsilon and deterministic.
    pub fn add_transition(
        &mut self,
        source: StateRef,
        symbol: Symbol,
        target: StateRef,
    ) -> Result<()> {
        let c = match symbol {
            Symbol::Epsilon => {
                return Err(AutomatonError::EpsilonNotAllowedInDfa);
            }
            Symbol::Symbol(c) => c,
        };

        self.automaton.require_state(&source)?;
        self.automaton.require_state(&target)?;

        if self.transitions.contains_key(&(source.clone(), c)) {
            return Err(AutomatonError::DuplicateDfaTransition {
                from: source,
                symbol,
            });
        }

        self.symbols.insert(symbol);
        self.transitions.insert((source, c), target);

        Ok(())
    }

    pub fn next(&self, state: &StateRef, c: char) -> Option<StateRef> {
        self.transitions.get(&(state.clone(), c)).cloned()
    }

    pub fn execute(&self, word: &str) -> bool {
        let mut current = self.initial();

        for c in word.chars() {
            match self.next(&current, c) {
                Some(next) => current = next,
                None => return false,
            }
        }

        self.is_accepting(&current)
    }

    pub fn completed(mut self) -> Self {
        let alphabet: Vec<char> = self
            .symbols
            .iter()
            .map(|s| match s {
                Symbol::Symbol(c) => *c,
                Symbol::Epsilon => {
                    unreachable!("a DFA can never contain an epsilon symbol")
                }
            })
            .collect();

        if alphabet.is_empty() {
            return self;
        }

        let states: Vec<StateRef> = self.states().collect();
        let mut missing = Vec::new();
        for state in &states {
            for &c in &alphabet {
                if !self.transitions.contains_key(&(state.clone(), c)) {
                    missing.push((state.clone(), c));
                }
            }
        }

        if missing.is_empty() {
            return self;
        }

        let trap = self.automaton.add_fresh_state();
        for &c in &alphabet {
            self.transitions.insert((trap.clone(), c), trap.clone());
        }
        for (state, c) in missing {
            self.transitions.insert((state, c), trap.clone());
        }

        self
    }

    pub fn to_graph(&self) -> graphviz_rust::dot_structures::Graph {
        let edges = self
            .transitions
            .iter()
            .map(|((source, c), target)| {
                (source.clone(), Symbol::from(*c), target.clone())
            })
            .collect();

        render_graph(
            self.states().collect(),
            self.automaton.accepting(),
            &self.automaton.initial(),
            edges,
        )
    }
}

#[derive(Debug)]
pub struct NFA {
    automaton: Automaton,
}

impl NFA {
    pub fn new(initial_name: impl Into<String>) -> Self {
        Self {
            automaton: Automaton::new(initial_name),
        }
    }

    pub fn initial(&self) -> StateRef {
        self.automaton.initial()
    }

    pub fn states(&self) -> impl Iterator<Item = StateRef> + '_ {
        self.automaton.states()
    }

    pub fn alphabet(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.automaton.alphabet()
    }

    pub fn is_accepting(&self, state: &StateRef) -> bool {
        self.automaton.is_accepting(state)
    }

    pub fn add_state(&mut self, name: impl Into<String>) -> Result<StateRef> {
        self.automaton.add_state(name)
    }

    pub fn mark_state_accepting(&mut self, state: StateRef) -> Result<()> {
        self.automaton.mark_state_accepting(state)
    }

    pub fn add_transition(
        &mut self,
        source: StateRef,
        symbol: Symbol,
        target: StateRef,
    ) -> Result<()> {
        self.automaton.add_transition(source, symbol, target)
    }

    pub fn epsilon_closure(
        &self,
        sources: impl IntoIterator<Item = StateRef>,
    ) -> HashSet<StateRef> {
        let mut closure: HashSet<StateRef> = sources.into_iter().collect();
        let mut queue: VecDeque<StateRef> = closure.iter().cloned().collect();

        while let Some(state) = queue.pop_front() {
            let Some(by_symbol) = self.automaton.transitions.get(&state) else {
                continue;
            };

            let Some(targets) = by_symbol.get(&Symbol::Epsilon) else {
                continue;
            };

            for target in targets {
                if closure.insert(target.clone()) {
                    queue.push_back(target.clone());
                }
            }
        }

        closure
    }

    pub fn initial_configuration(&self) -> HashSet<StateRef> {
        self.epsilon_closure([self.initial().clone()])
    }

    pub fn reachable_from(
        &self,
        sources: impl IntoIterator<Item = StateRef>,
        symbol: &Symbol,
    ) -> HashSet<StateRef> {
        debug_assert!(
            !matches!(symbol, Symbol::Epsilon),
            "reachable_from is for non-epsilon symbols"
        );
        let targets = self.automaton.reachable_from(sources, symbol);
        self.epsilon_closure(targets)
    }

    pub fn to_dfa(&self) -> DFA {
        let initial_subset = self.initial_configuration();
        let mut dfa = DFA::new(subset_name(&initial_subset));
        let dfa_initial = dfa.initial();

        if initial_subset.iter().any(|s| self.is_accepting(s)) {
            dfa.mark_state_accepting(dfa_initial.clone())
                .expect("the DFA's own initial state always exists in it");
        }

        let mut subsets: HashMap<String, StateRef> = HashMap::new();
        subsets.insert(subset_name(&initial_subset), dfa_initial.clone());

        let mut queue = VecDeque::from([(initial_subset, dfa_initial)]);
        let alphabet: Vec<Symbol> = self.alphabet().collect();

        while let Some((subset, dfa_state)) = queue.pop_front() {
            for symbol in &alphabet {
                let next_subset =
                    self.reachable_from(subset.iter().cloned(), symbol);

                if next_subset.is_empty() {
                    continue;
                }

                let key = subset_name(&next_subset);
                let target = match subsets.get(&key) {
                    Some(existing) => existing.clone(),
                    None => {
                        let state = dfa
                            .add_state(key.clone())
                            .expect("subset names are unique by construction");

                        if next_subset.iter().any(|s| self.is_accepting(s)) {
                            dfa.mark_state_accepting(state.clone())
                                .expect("state was just added to this DFA");
                        }

                        subsets.insert(key, state.clone());
                        queue.push_back((next_subset, state.clone()));
                        state
                    }
                };

                dfa.add_transition(dfa_state.clone(), symbol.clone(), target)
                    .expect("subset construction visits each (state, symbol) pair once");
            }
        }

        dfa
    }

    pub fn to_graph(&self) {
        self.automaton.to_graph();
    }
}

fn subset_name(states: &HashSet<StateRef>) -> String {
    let mut names: Vec<String> = states
        .iter()
        .map(|s| escape_subset_member(s.name()))
        .collect();
    names.sort_unstable();
    format!("{{{}}}", names.join(","))
}

fn escape_subset_member(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ',' => out.push_str("\\,"),
            _ => out.push(c),
        }
    }
    out
}

fn render_graph(
    mut states: Vec<StateRef>,
    accepting: &HashSet<StateRef>,
    initial: &StateRef,
    edges: Vec<(StateRef, Symbol, StateRef)>,
) -> graphviz_rust::dot_structures::Graph {
    use graphviz_rust::parse;

    states.sort_by(|a, b| a.name().cmp(b.name()));

    let ids: HashMap<StateRef, String> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), format!("n{i}")))
        .collect();

    let mut dot = String::from("digraph automaton {\n  graph[rankdir=LR]\n");

    for state in &states {
        let shape = if accepting.contains(state) {
            "doubleoctagon"
        } else {
            "box"
        };
        dot.push_str(&format!(
            "  {} [label=\"{}\" shape={}]\n",
            ids[state],
            escape_dot_string(state.name()),
            shape
        ));
    }

    dot.push_str("  start [shape=none label=\"\"]\n");
    dot.push_str(&format!("  start -> {}\n", ids[initial]));

    // Group parallel edges between the same pair of states into one
    // comma-separated label, as the original did.
    let mut grouped: HashMap<(StateRef, StateRef), Vec<Symbol>> =
        HashMap::new();
    for (source, symbol, target) in edges {
        grouped.entry((source, target)).or_default().push(symbol);
    }

    let mut grouped: Vec<((StateRef, StateRef), Vec<Symbol>)> =
        grouped.into_iter().collect();
    grouped.sort_by(|a, b| {
        let ((asrc, atgt), _) = a;
        let ((bsrc, btgt), _) = b;
        (asrc.name(), atgt.name()).cmp(&(bsrc.name(), btgt.name()))
    });

    for ((source, target), mut symbols) in grouped {
        symbols.sort();
        let label = symbols
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        dot.push_str(&format!(
            "  {} -> {} [label=\"{}\"]\n",
            ids[&source],
            ids[&target],
            escape_dot_string(&label)
        ));
    }

    dot.push_str("}\n");

    parse(&dot).expect("generated automaton DOT should always be valid")
}

fn escape_dot_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(c: char) -> Symbol {
        Symbol::from(c)
    }

    fn eps() -> Symbol {
        Symbol::Epsilon
    }

    #[test]
    fn new_automaton_has_initial_state() {
        let automaton = Automaton::new("start");

        assert_eq!(automaton.states().count(), 1);
        assert_eq!(automaton.initial().to_string(), "start");
        assert!(!automaton.is_accepting(&automaton.initial()));
    }

    #[test]
    fn add_state_adds_state() {
        let mut automaton = Automaton::new("start");

        let state = automaton.add_state("end").unwrap();

        assert_eq!(state.to_string(), "end");
        assert_eq!(automaton.states().count(), 2);
        assert!(automaton.states().any(|s| s == state));
    }

    #[test]
    fn add_state_rejects_duplicate_state() {
        let mut automaton = Automaton::new("start");

        automaton.add_state("foo").unwrap();
        let err = automaton.add_state("foo").unwrap_err();

        assert_eq!(err, AutomatonError::DuplicateState("foo".into()));
    }

    #[test]
    fn mark_state_accepting() {
        let mut automaton = Automaton::new("start");
        let state = automaton.add_state("accept").unwrap();

        assert!(!automaton.is_accepting(&state));

        automaton.mark_state_accepting(state.clone()).unwrap();

        assert!(automaton.is_accepting(&state));
    }

    #[test]
    fn cannot_mark_foreign_state_accepting() {
        let mut first = Automaton::new("first");
        let mut second = Automaton::new("second");

        let state = first.add_state("state").unwrap();

        let err = second.mark_state_accepting(state).unwrap_err();
        assert!(matches!(err, AutomatonError::UnknownState(_)));
    }

    #[test]
    fn add_transition_and_reachable_from() {
        let mut automaton = Automaton::new("start");
        let initial = automaton.initial();
        let middle = automaton.add_state("middle").unwrap();
        let end = automaton.add_state("end").unwrap();

        let a = sym('a');

        automaton
            .add_transition(initial.clone(), a.clone(), middle.clone())
            .unwrap();
        automaton
            .add_transition(middle.clone(), a.clone(), end.clone())
            .unwrap();

        let reachable = automaton.reachable_from([initial.clone()], &a);
        assert_eq!(reachable.len(), 1);
        assert!(reachable.contains(&middle));

        let reachable = automaton.reachable_from([middle.clone()], &a);
        assert_eq!(reachable.len(), 1);
        assert!(reachable.contains(&end));
    }

    #[test]
    fn alphabet_excludes_epsilon() {
        let mut automaton = Automaton::new("start");
        let initial = automaton.initial();
        let end = automaton.add_state("end").unwrap();

        automaton
            .add_transition(initial.clone(), eps(), end.clone())
            .unwrap();
        automaton
            .add_transition(initial.clone(), sym('a'), end.clone())
            .unwrap();
        automaton.add_transition(initial, sym('b'), end).unwrap();

        let mut alphabet: Vec<char> = automaton
            .alphabet()
            .filter_map(|s| match s {
                Symbol::Symbol(c) => Some(c),
                Symbol::Epsilon => None,
            })
            .collect();

        alphabet.sort_unstable();

        assert_eq!(alphabet, vec!['a', 'b']);
    }

    #[test]
    fn epsilon_closure_contains_source_states() {
        let nfa = NFA::new("start");
        let initial = nfa.initial();

        let closure = nfa.epsilon_closure([initial.clone()]);

        assert_eq!(closure.len(), 1);
        assert!(closure.contains(&initial));
    }

    #[test]
    fn epsilon_closure_follows_epsilon_transitions() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();

        let q1 = nfa.add_state("one").unwrap();
        let q2 = nfa.add_state("two").unwrap();

        nfa.add_transition(initial.clone(), eps(), q1.clone())
            .unwrap();
        nfa.add_transition(q1.clone(), eps(), q2.clone()).unwrap();

        let closure = nfa.initial_configuration();

        assert_eq!(closure.len(), 3);
        assert!(closure.contains(&initial));
        assert!(closure.contains(&q1));
        assert!(closure.contains(&q2));
    }

    #[test]
    fn epsilon_closure_handles_cycles() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();

        let q1 = nfa.add_state("one").unwrap();
        let q2 = nfa.add_state("two").unwrap();

        nfa.add_transition(initial.clone(), eps(), q1.clone())
            .unwrap();
        nfa.add_transition(q1.clone(), eps(), q2.clone()).unwrap();
        nfa.add_transition(q2.clone(), eps(), q1.clone()).unwrap();

        let closure = nfa.initial_configuration();

        assert_eq!(closure.len(), 3);
        assert!(closure.contains(&initial));
        assert!(closure.contains(&q1));
        assert!(closure.contains(&q2));
    }

    #[test]
    fn epsilon_closure_from_multiple_sources() {
        let mut nfa = NFA::new("start");

        let q1 = nfa.add_state("one").unwrap();
        let q2 = nfa.add_state("two").unwrap();
        let q3 = nfa.add_state("three").unwrap();

        nfa.add_transition(q1.clone(), eps(), q3.clone()).unwrap();
        nfa.add_transition(q2.clone(), eps(), q3.clone()).unwrap();

        let closure = nfa.epsilon_closure([q1.clone(), q2.clone()]);

        assert_eq!(closure.len(), 3);
        assert!(closure.contains(&q1));
        assert!(closure.contains(&q2));
        assert!(closure.contains(&q3));
    }

    #[test]
    fn nfa_reachable_from_includes_epsilon_closure() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();

        let q1 = nfa.add_state("one").unwrap();
        let q2 = nfa.add_state("two").unwrap();

        nfa.add_transition(initial.clone(), sym('a'), q1.clone())
            .unwrap();
        nfa.add_transition(q1.clone(), eps(), q2.clone()).unwrap();

        let reachable = nfa.reachable_from([initial], &sym('a'));

        assert_eq!(reachable.len(), 2);
        assert!(reachable.contains(&q1));
        assert!(reachable.contains(&q2));
    }

    #[test]
    fn dfa_next_returns_target() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial();
        let end = dfa.add_state("end").unwrap();

        dfa.add_transition(initial.clone(), sym('a'), end.clone())
            .unwrap();

        assert_eq!(dfa.next(&initial, 'a'), Some(end));
    }

    #[test]
    fn dfa_next_returns_none_for_missing_transition() {
        let dfa = DFA::new("start");
        let initial = dfa.initial();

        assert_eq!(dfa.next(&initial, 'a'), None);
    }

    #[test]
    fn dfa_execute_accepts_valid_word() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial();
        let accept = dfa.add_state("accept").unwrap();

        dfa.add_transition(initial, sym('a'), accept.clone())
            .unwrap();
        dfa.mark_state_accepting(accept).unwrap();

        assert!(dfa.execute("a"));
    }

    #[test]
    fn dfa_execute_rejects_invalid_word() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial();
        let accept = dfa.add_state("accept").unwrap();

        dfa.add_transition(initial, sym('a'), accept.clone())
            .unwrap();
        dfa.mark_state_accepting(accept).unwrap();

        assert!(!dfa.execute(""));
        assert!(!dfa.execute("b"));
        assert!(!dfa.execute("aa"));
    }

    #[test]
    fn dfa_execute_handles_multiple_symbols() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial();
        let one = dfa.add_state("one").unwrap();
        let two = dfa.add_state("two").unwrap();
        let accept = dfa.add_state("accept").unwrap();

        dfa.add_transition(initial, sym('a'), one.clone()).unwrap();
        dfa.add_transition(one, sym('b'), two.clone()).unwrap();
        dfa.add_transition(two, sym('c'), accept.clone()).unwrap();
        dfa.mark_state_accepting(accept).unwrap();

        assert!(dfa.execute("abc"));
        assert!(!dfa.execute("ab"));
        assert!(!dfa.execute("abd"));
    }

    #[test]
    fn dfa_rejects_epsilon_transition() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial();
        let end = dfa.add_state("end").unwrap();

        let err = dfa.add_transition(initial, eps(), end).unwrap_err();
        assert_eq!(err, AutomatonError::EpsilonNotAllowedInDfa);
    }

    #[test]
    fn dfa_rejects_nondeterministic_transition() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial();
        let first = dfa.add_state("first").unwrap();
        let second = dfa.add_state("second").unwrap();

        dfa.add_transition(initial.clone(), sym('a'), first)
            .unwrap();
        let err = dfa.add_transition(initial, sym('a'), second).unwrap_err();

        assert!(matches!(err, AutomatonError::DuplicateDfaTransition { .. }));
    }

    #[test]
    fn completed_dfa_is_total_and_traps_correctly() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial();
        let accept = dfa.add_state("accept").unwrap();

        dfa.add_transition(initial.clone(), sym('a'), accept.clone())
            .unwrap();
        dfa.add_transition(accept.clone(), sym('b'), accept.clone())
            .unwrap();
        dfa.mark_state_accepting(accept).unwrap();

        let dfa = dfa.completed();

        assert!(dfa.next(&initial, 'b').is_some());
        assert!(!dfa.execute("b"));
        assert!(!dfa.execute("ba"));
        assert!(dfa.execute("ab"));
    }

    #[test]
    fn subset_name_is_deterministic() {
        let mut nfa = NFA::new("start");
        let b = nfa.add_state("b").unwrap();
        let a = nfa.add_state("a").unwrap();

        let states: HashSet<StateRef> = [b, a].into_iter().collect();

        assert_eq!(subset_name(&states), "{a,b}");
    }

    #[test]
    fn subset_name_handles_single_state() {
        let mut nfa = NFA::new("start");
        let state = nfa.add_state("foo").unwrap();

        let states: HashSet<StateRef> = [state].into_iter().collect();

        assert_eq!(subset_name(&states), "{foo}");
    }

    #[test]
    fn subset_name_handles_empty_set() {
        let states: HashSet<StateRef> = HashSet::new();

        assert_eq!(subset_name(&states), "{}");
    }

    #[test]
    fn subset_name_does_not_collide_on_commas_in_names() {
        let mut nfa1 = NFA::new("a");
        let b = nfa1.add_state("b").unwrap();
        let set1: HashSet<StateRef> = [nfa1.initial(), b].into_iter().collect();

        let nfa2 = NFA::new("a,b");
        let set2: HashSet<StateRef> = [nfa2.initial()].into_iter().collect();

        assert_ne!(subset_name(&set1), subset_name(&set2));
    }

    #[test]
    fn nfa_to_dfa_converts_simple_nfa() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();
        let accept = nfa.add_state("accept").unwrap();

        nfa.add_transition(initial, sym('a'), accept.clone())
            .unwrap();
        nfa.mark_state_accepting(accept).unwrap();

        let dfa = nfa.to_dfa();

        assert_eq!(dfa.states().count(), 2);
        assert!(dfa.execute("a"));
        assert!(!dfa.execute(""));
        assert!(!dfa.execute("b"));
        assert!(!dfa.execute("aa"));
    }

    #[test]
    fn nfa_to_dfa_preserves_epsilon_acceptance() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();
        let accept = nfa.add_state("accept").unwrap();

        nfa.add_transition(initial, eps(), accept.clone()).unwrap();
        nfa.mark_state_accepting(accept).unwrap();

        let dfa = nfa.to_dfa();

        assert!(dfa.is_accepting(&dfa.initial()));
        assert!(dfa.execute(""));
    }

    #[test]
    fn nfa_to_dfa_handles_nondeterminism() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();
        let left = nfa.add_state("left").unwrap();
        let right = nfa.add_state("right").unwrap();
        let accept = nfa.add_state("accept").unwrap();

        nfa.add_transition(initial.clone(), sym('a'), left.clone())
            .unwrap();
        nfa.add_transition(initial, sym('a'), right.clone())
            .unwrap();
        nfa.add_transition(left, sym('b'), accept.clone()).unwrap();
        nfa.add_transition(right, sym('c'), accept.clone()).unwrap();
        nfa.mark_state_accepting(accept).unwrap();

        let dfa = nfa.to_dfa();

        assert!(dfa.execute("ab"));
        assert!(dfa.execute("ac"));
        assert!(!dfa.execute("a"));
        assert!(!dfa.execute("aa"));
        assert!(!dfa.execute("abc"));
    }

    #[test]
    fn nfa_to_dfa_handles_epsilon_and_nondeterminism() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();
        let left = nfa.add_state("left").unwrap();
        let right = nfa.add_state("right").unwrap();
        let accept = nfa.add_state("accept").unwrap();

        nfa.add_transition(initial.clone(), eps(), left.clone())
            .unwrap();
        nfa.add_transition(left, sym('a'), right.clone()).unwrap();
        nfa.add_transition(initial, sym('a'), accept.clone())
            .unwrap();
        nfa.mark_state_accepting(right).unwrap();
        nfa.mark_state_accepting(accept).unwrap();

        let dfa = nfa.to_dfa();

        assert!(dfa.execute("a"));
        assert!(!dfa.execute(""));
        assert!(!dfa.execute("aa"));
    }

    #[test]
    fn nfa_to_dfa_does_not_create_unreachable_states() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();
        let reachable = nfa.add_state("reachable").unwrap();
        let unreachable = nfa.add_state("unreachable").unwrap();

        nfa.add_transition(initial, sym('a'), reachable).unwrap();
        nfa.mark_state_accepting(unreachable).unwrap();

        let dfa = nfa.to_dfa();

        let names: HashSet<String> =
            dfa.states().map(|state| state.to_string()).collect();

        assert!(names.contains("{start}"));
        assert!(names.contains("{reachable}"));
        assert!(!names.iter().any(|name| name.contains("unreachable")));
        assert!(!dfa.execute(""));
        assert!(!dfa.execute("a"));
    }

    #[test]
    fn nfa_to_dfa_can_recognize_empty_word() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();
        nfa.mark_state_accepting(initial).unwrap();

        let dfa = nfa.to_dfa();

        assert!(dfa.execute(""));
        assert!(!dfa.execute("a"));
    }

    #[test]
    fn nfa_to_dfa_produces_stable_subset_names() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial();
        let b = nfa.add_state("b").unwrap();
        let a = nfa.add_state("a").unwrap();

        nfa.add_transition(initial.clone(), sym('x'), b).unwrap();
        nfa.add_transition(initial, sym('x'), a).unwrap();

        let dfa = nfa.to_dfa();

        let names: HashSet<String> =
            dfa.states().map(|state| state.to_string()).collect();

        assert!(names.contains("{start}"));
        assert!(names.contains("{a,b}"));
    }

    #[test]
    fn nfa_to_dfa_recognizes_ends_with_ab() {
        let mut nfa = NFA::new("q0");
        let q0 = nfa.initial();
        let q1 = nfa.add_state("s1").unwrap();
        let q2 = nfa.add_state("s2").unwrap();

        nfa.add_transition(q0.clone(), sym('a'), q0.clone())
            .unwrap();
        nfa.add_transition(q0.clone(), sym('b'), q0.clone())
            .unwrap();
        nfa.add_transition(q0.clone(), sym('a'), q1.clone())
            .unwrap();
        nfa.add_transition(q1, sym('b'), q2.clone()).unwrap();
        nfa.mark_state_accepting(q2).unwrap();

        let dfa = nfa.to_dfa();

        assert!(dfa.execute("ab"));
        assert!(dfa.execute("aab"));
        assert!(dfa.execute("bbab"));
        assert!(!dfa.execute("ba"));
        assert!(!dfa.execute(""));
        assert!(!dfa.execute("abb"));
    }

    #[test]
    fn to_graph_does_not_panic_on_adversarial_names() {
        let mut nfa = NFA::new("start");
        let weird = nfa.add_state("wei\"rd\\name").unwrap();

        nfa.add_transition(nfa.initial(), sym('x'), weird.clone())
            .unwrap();
        nfa.mark_state_accepting(weird).unwrap();

        let _ = nfa.to_graph();
    }
}
