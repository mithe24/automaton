use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    rc::Rc,
};

pub type StateRef = Rc<State>;
pub type SymbolRef = Rc<Symbol>;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct State {
    name: Rc<str>,
}

impl State {
    pub fn new(name: impl Into<Rc<str>>) -> Self {
        Self { name: name.into() }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum Symbol {
    Epsilon,
    Symbol(char),
}

// These are super unidiomatic,
// but are here to mirror the orginal API more closely
impl Symbol {
    pub fn epsilon() -> SymbolRef {
        Rc::new(Self::Epsilon)
    }

    pub fn char(c: char) -> SymbolRef {
        Rc::new(Self::Symbol(c))
    }

    pub fn is_epsilon(&self) -> bool {
        matches!(self, Self::Epsilon)
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

#[derive(Debug)]
pub struct Automaton {
    states: Vec<StateRef>,
    accepting: HashSet<StateRef>,
    symbols: HashSet<SymbolRef>,
    initial: StateRef,
    transitions: HashMap<StateRef, HashMap<SymbolRef, HashSet<StateRef>>>,
}

impl Automaton {
    pub fn new(initial_name: impl Into<Rc<str>>) -> Self {
        let initial = Rc::new(State::new(initial_name));
        Self {
            states: vec![initial.clone()],
            accepting: HashSet::new(),
            symbols: HashSet::new(),
            initial,
            transitions: HashMap::new(),
        }
    }

    pub fn initial(&self) -> StateRef {
        self.initial.clone()
    }

    pub fn states(&self) -> impl Iterator<Item = StateRef> {
        self.states.iter().cloned()
    }

    /// Adds a new state and returns its name.
    /// If a name is provided, it must not start with 'q'.
    pub fn add_state(&mut self, name: impl Into<Rc<str>>) -> StateRef {
        let name = name.into();
        assert!(
            !name.starts_with('q'),
            "state names cannot start with 'q': {name}"
        );

        let state = Rc::new(State::new(name));
        assert!(
            !self.states.iter().any(|existing| existing == &state),
            "state already exists: {}",
            state
        );
        self.states.push(state.clone());
        state
    }

    pub fn alphabet(&self) -> impl Iterator<Item = SymbolRef> {
        self.symbols
            .iter()
            .filter(|symbol| !matches!(symbol.as_ref(), Symbol::Epsilon))
            .cloned()
    }

    pub fn is_accepting(&self, state: StateRef) -> bool {
        self.accepting.contains(&state)
    }

    pub fn mark_state_accepting(&mut self, state: StateRef) {
        self.assert_has_state(state.clone());
        self.accepting.insert(state.clone());
    }

    /// Adds a transition between two states for the symbol provided;
    /// states have to exist.
    pub fn add_transition(
        &mut self,
        source: StateRef,
        symbol: SymbolRef,
        target: StateRef,
    ) {
        self.assert_has_state(source.clone());
        self.assert_has_state(target.clone());

        self.transitions
            .entry(source.clone())
            .or_default()
            .entry(symbol.clone())
            .or_default()
            .insert(target.clone());
        self.symbols.insert(symbol.clone());
    }

    pub fn all_transition(
        &self,
    ) -> impl Iterator<Item = (StateRef, SymbolRef, StateRef)> {
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
        symbol: SymbolRef,
    ) -> HashSet<StateRef> {
        sources
            .into_iter()
            .flat_map(|source| {
                self.transitions
                    .get(&source)
                    .and_then(|by_symbol| by_symbol.get(&symbol))
                    .into_iter()
                    .flatten()
                    .cloned()
            })
            .collect()
    }

    fn assert_has_state(&self, state: StateRef) {
        assert!(
            self.states.iter().any(|s| s == &state),
            "state does not belong to this automaton: {state}"
        );
    }

    pub fn to_graph(&self) {
        todo!()
    }
}

#[derive(Debug)]
pub struct DFA {
    automaton: Automaton,
}

impl DFA {
    pub fn new(initial_name: impl Into<Rc<str>>) -> Self {
        Self {
            automaton: Automaton::new(initial_name),
        }
    }

    pub fn initial(&self) -> StateRef {
        self.automaton.initial()
    }

    pub fn states(&self) -> impl Iterator<Item = StateRef> {
        self.automaton.states()
    }

    pub fn alphabet(&self) -> impl Iterator<Item = SymbolRef> {
        self.automaton.alphabet()
    }

    pub fn is_accepting(&self, state: StateRef) -> bool {
        self.automaton.is_accepting(state)
    }

    pub fn add_state(&mut self, name: impl Into<Rc<str>>) -> StateRef {
        self.automaton.add_state(name)
    }

    pub fn mark_state_accepting(&mut self, state: StateRef) {
        self.automaton.mark_state_accepting(state);
    }

    /// DFA transitions must be non-epsilon and deterministic.
    pub fn add_transition(
        &mut self,
        source: StateRef,
        symbol: SymbolRef,
        target: StateRef,
    ) {
        assert!(
            !symbol.is_epsilon(),
            "DFAs cannot contain epsilon transitions"
        );

        let already_exists = self
            .automaton
            .transitions
            .get(&source)
            .and_then(|m| m.get(&symbol))
            .is_some_and(|targets| !targets.is_empty());

        assert!(
            !already_exists,
            "DFA transition already exists for {source} / {symbol}"
        );

        self.automaton.add_transition(source, symbol, target);
    }

    pub fn next(&self, state: StateRef, symbol: SymbolRef) -> Option<StateRef> {
        self.automaton
            .transitions
            .get(&state)
            .and_then(|m| m.get(&symbol))
            .and_then(|targets| targets.iter().next())
            .cloned()
    }

    pub fn execute(&self, word: &str) -> bool {
        let mut current = self.initial().clone();

        for c in word.chars() {
            let symbol = Symbol::char(c);
            match self.next(current, symbol) {
                Some(next) => current = next,
                None => return false,
            }
        }

        self.is_accepting(current)
    }

    pub fn to_graph(&self) {
        self.automaton.to_graph()
    }
}

#[derive(Debug)]
pub struct NFA {
    automaton: Automaton,
}

impl NFA {
    pub fn new(initial_name: impl Into<Rc<str>>) -> Self {
        Self {
            automaton: Automaton::new(initial_name),
        }
    }

    pub fn initial(&self) -> StateRef {
        self.automaton.initial()
    }

    pub fn states(&self) -> impl Iterator<Item = StateRef> {
        self.automaton.states()
    }

    pub fn alphabet(&self) -> impl Iterator<Item = SymbolRef> {
        self.automaton.alphabet()
    }

    pub fn is_accepting(&self, state: StateRef) -> bool {
        self.automaton.is_accepting(state)
    }

    pub fn add_state(&mut self, name: impl Into<Rc<str>>) -> StateRef {
        self.automaton.add_state(name)
    }

    pub fn mark_state_accepting(&mut self, state: StateRef) {
        self.automaton.mark_state_accepting(state);
    }

    pub fn add_transition(
        &mut self,
        source: StateRef,
        symbol: SymbolRef,
        target: StateRef,
    ) {
        self.automaton.add_transition(source, symbol, target);
    }

    pub fn epsilon_closure(
        &self,
        sources: impl IntoIterator<Item = StateRef>,
    ) -> HashSet<StateRef> {
        let mut closure: HashSet<StateRef> = sources.into_iter().collect();
        let mut queue: VecDeque<StateRef> = closure.iter().cloned().collect();

        let epsilon = Symbol::epsilon();

        while let Some(state) = queue.pop_front() {
            let Some(by_symbol) = self.automaton.transitions.get(&state) else {
                continue;
            };

            let Some(targets) = by_symbol.get(&epsilon) else {
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
        symbol: SymbolRef,
    ) -> HashSet<StateRef> {
        assert!(!matches!(symbol.as_ref(), Symbol::Epsilon));
        let targets = self.automaton.reachable_from(sources, symbol);
        self.epsilon_closure(targets)
    }

    pub fn to_dfa(&self) -> DFA {
        let initial_subset = self.initial_configuration();
        let initial_name = subset_name(&initial_subset);

        let mut dfa = DFA::new(initial_name);
        let dfa_initial = dfa.initial().clone();

        if initial_subset
            .iter()
            .any(|s| self.is_accepting(Rc::clone(s)))
        {
            dfa.mark_state_accepting(dfa_initial.clone());
        }

        let mut subsets: HashMap<String, StateRef> = HashMap::new();
        subsets.insert(subset_name(&initial_subset), dfa_initial.clone());

        let mut queue = VecDeque::from([(initial_subset, dfa_initial)]);
        let alphabet: Vec<SymbolRef> = self.alphabet().collect();

        while let Some((subset, dfa_state)) = queue.pop_front() {
            for symbol in &alphabet {
                let next_subset =
                    self.reachable_from(subset.iter().cloned(), symbol.clone());

                if next_subset.is_empty() {
                    continue;
                }

                let key = subset_name(&next_subset);
                let target = match subsets.get(&key) {
                    Some(existing) => existing.clone(),
                    None => {
                        let state = dfa.add_state(key.clone());

                        if next_subset
                            .iter()
                            .any(|s| self.is_accepting(s.clone()))
                        {
                            dfa.mark_state_accepting(state.clone());
                        }

                        subsets.insert(key, state.clone());
                        queue.push_back((next_subset, state.clone()));

                        state
                    }
                };
                dfa.add_transition(dfa_state.clone(), symbol.clone(), target);
            }
        }

        dfa
    }

    pub fn to_graph(&self) {
        self.automaton.to_graph();
    }
}

fn subset_name(states: &HashSet<StateRef>) -> String {
    let mut names: Vec<&str> =
        states.iter().map(|state| state.name.as_ref()).collect();

    names.sort_unstable();

    format!("{{{}}}", names.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(c: char) -> SymbolRef {
        Symbol::char(c)
    }

    fn eps() -> SymbolRef {
        Symbol::epsilon()
    }

    #[test]
    fn new_automaton_has_initial_state() {
        let automaton = Automaton::new("start");

        assert_eq!(automaton.states().count(), 1);
        assert_eq!(automaton.initial().to_string(), "start");
        assert!(!automaton.is_accepting(automaton.initial()));
    }

    #[test]
    fn add_state_adds_state() {
        let mut automaton = Automaton::new("start");

        let state = automaton.add_state("end");

        assert_eq!(state.to_string(), "end");
        assert_eq!(automaton.states().count(), 2);
        assert!(automaton.states().any(|s| s == state));
    }

    #[test]
    #[should_panic(expected = "state names cannot start with 'q'")]
    fn add_state_rejects_names_starting_with_q() {
        let mut automaton = Automaton::new("start");

        automaton.add_state("q1");
    }

    #[test]
    #[should_panic(expected = "state already exists")]
    fn add_state_rejects_duplicate_state() {
        let mut automaton = Automaton::new("start");

        automaton.add_state("foo");
        automaton.add_state("foo");
    }

    #[test]
    fn mark_state_accepting() {
        let mut automaton = Automaton::new("start");
        let state = automaton.add_state("accept");

        assert!(!automaton.is_accepting(state.clone()));

        automaton.mark_state_accepting(state.clone());

        assert!(automaton.is_accepting(state.clone()));
    }

    #[test]
    #[should_panic(expected = "state does not belong to this automaton")]
    fn cannot_mark_foreign_state_accepting() {
        let mut first = Automaton::new("first");
        let mut second = Automaton::new("second");

        let state = first.add_state("state");

        second.mark_state_accepting(state.clone());
    }

    #[test]
    fn add_transition_and_reachable_from() {
        let mut automaton = Automaton::new("start");
        let initial = automaton.initial().clone();
        let middle = automaton.add_state("middle");
        let end = automaton.add_state("end");

        let a = sym('a');

        automaton.add_transition(initial.clone(), a.clone(), middle.clone());
        automaton.add_transition(middle.clone(), a.clone(), end.clone());

        let reachable = automaton.reachable_from([initial.clone()], a.clone());

        assert_eq!(reachable.len(), 1);
        assert!(reachable.contains(&middle));

        let reachable = automaton.reachable_from([middle.clone()], a.clone());

        assert_eq!(reachable.len(), 1);
        assert!(reachable.contains(&end));
    }

    #[test]
    fn alphabet_excludes_epsilon() {
        let mut automaton = Automaton::new("start");
        let initial = automaton.initial().clone();
        let end = automaton.add_state("end");

        automaton.add_transition(initial.clone(), eps(), end.clone());
        automaton.add_transition(initial.clone(), sym('a'), end.clone());
        automaton.add_transition(initial.clone(), sym('b'), end.clone());

        let mut alphabet: Vec<char> = automaton
            .alphabet()
            .filter_map(|s| match s.as_ref() {
                Symbol::Symbol(c) => Some(*c),
                Symbol::Epsilon => None,
            })
            .collect();

        alphabet.sort_unstable();

        assert_eq!(alphabet, vec!['a', 'b']);
    }

    #[test]
    fn epsilon_closure_contains_source_states() {
        let nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let closure = nfa.epsilon_closure([initial.clone()]);

        assert_eq!(closure.len(), 1);
        assert!(closure.contains(&initial));
    }

    #[test]
    fn epsilon_closure_follows_epsilon_transitions() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let q1 = nfa.add_state("one");
        let q2 = nfa.add_state("two");

        let epsilon = eps();

        nfa.add_transition(initial.clone(), epsilon.clone(), q1.clone());
        nfa.add_transition(q1.clone(), epsilon.clone(), q2.clone());

        let closure = nfa.initial_configuration();

        assert_eq!(closure.len(), 3);
        assert!(closure.contains(&initial));
        assert!(closure.contains(&q1));
        assert!(closure.contains(&q2));
    }

    #[test]
    fn epsilon_closure_handles_cycles() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let q1 = nfa.add_state("one");
        let q2 = nfa.add_state("two");

        let epsilon = eps();

        nfa.add_transition(initial.clone(), epsilon.clone(), q1.clone());
        nfa.add_transition(q1.clone(), epsilon.clone(), q2.clone());
        nfa.add_transition(q2.clone(), epsilon.clone(), q1.clone());

        let closure = nfa.initial_configuration();

        assert_eq!(closure.len(), 3);
        assert!(closure.contains(&initial));
        assert!(closure.contains(&q1));
        assert!(closure.contains(&q2));
    }

    #[test]
    fn epsilon_closure_from_multiple_sources() {
        let mut nfa = NFA::new("start");

        let q1 = nfa.add_state("one");
        let q2 = nfa.add_state("two");
        let q3 = nfa.add_state("three");

        let epsilon = eps();

        nfa.add_transition(q1.clone(), epsilon.clone(), q3.clone());
        nfa.add_transition(q2.clone(), epsilon.clone(), q3.clone());

        let closure = nfa.epsilon_closure([q1.clone(), q2.clone()]);

        assert_eq!(closure.len(), 3);
        assert!(closure.contains(&q1));
        assert!(closure.contains(&q2));
        assert!(closure.contains(&q3));
    }

    #[test]
    fn nfa_reachable_from_includes_epsilon_closure() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let q1 = nfa.add_state("one");
        let q2 = nfa.add_state("two");

        nfa.add_transition(initial.clone(), sym('a'), q1.clone());
        nfa.add_transition(q1.clone(), eps(), q2.clone());

        let reachable = nfa.reachable_from([initial], sym('a'));

        assert_eq!(reachable.len(), 2);
        assert!(reachable.contains(&q1));
        assert!(reachable.contains(&q2));
    }

    #[test]
    #[should_panic]
    fn nfa_reachable_from_rejects_epsilon() {
        let nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        nfa.reachable_from([initial], eps());
    }

    #[test]
    fn dfa_next_returns_target() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial().clone();

        let end = dfa.add_state("end");

        dfa.add_transition(initial.clone(), sym('a'), end.clone());

        let next = dfa.next(initial.clone(), sym('a'));

        assert_eq!(next, Some(end));
    }

    #[test]
    fn dfa_next_returns_none_for_missing_transition() {
        let dfa = DFA::new("start");
        let initial = dfa.initial().clone();

        assert_eq!(dfa.next(initial.clone(), sym('a')), None);
    }

    #[test]
    fn dfa_execute_accepts_valid_word() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial().clone();

        let accept = dfa.add_state("accept");

        dfa.add_transition(initial.clone(), sym('a'), accept.clone());
        dfa.mark_state_accepting(accept.clone());

        assert!(dfa.execute("a"));
    }

    #[test]
    fn dfa_execute_rejects_invalid_word() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial().clone();

        let accept = dfa.add_state("accept");

        dfa.add_transition(initial.clone(), sym('a'), accept.clone());
        dfa.mark_state_accepting(accept.clone());

        assert!(!dfa.execute(""));
        assert!(!dfa.execute("b"));
        assert!(!dfa.execute("aa"));
    }

    #[test]
    fn dfa_execute_handles_multiple_symbols() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial().clone();

        let one = dfa.add_state("one");
        let two = dfa.add_state("two");
        let accept = dfa.add_state("accept");

        dfa.add_transition(initial.clone(), sym('a'), one.clone());
        dfa.add_transition(one.clone(), sym('b'), two.clone());
        dfa.add_transition(two.clone(), sym('c'), accept.clone());

        dfa.mark_state_accepting(accept.clone());

        assert!(dfa.execute("abc"));
        assert!(!dfa.execute("ab"));
        assert!(!dfa.execute("abd"));
    }

    #[test]
    #[should_panic(expected = "DFAs cannot contain epsilon transitions")]
    fn dfa_rejects_epsilon_transition() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial().clone();

        let end = dfa.add_state("end");

        dfa.add_transition(initial.clone(), eps(), end.clone());
    }

    #[test]
    #[should_panic(expected = "DFA transition already exists")]
    fn dfa_rejects_nondeterministic_transition() {
        let mut dfa = DFA::new("start");
        let initial = dfa.initial().clone();

        let first = dfa.add_state("first");
        let second = dfa.add_state("second");

        dfa.add_transition(initial.clone(), sym('a'), first.clone());
        dfa.add_transition(initial.clone(), sym('a'), second.clone());
    }

    #[test]
    fn subset_name_is_deterministic() {
        let mut nfa = NFA::new("start");

        let b = nfa.add_state("b");
        let a = nfa.add_state("a");

        let states: HashSet<StateRef> = [b, a].into_iter().collect();

        assert_eq!(subset_name(&states), "{a,b}");
    }

    #[test]
    fn subset_name_handles_single_state() {
        let mut nfa = NFA::new("start");
        let state = nfa.add_state("foo");

        let states: HashSet<StateRef> = [state].into_iter().collect();

        assert_eq!(subset_name(&states), "{foo}");
    }

    #[test]
    fn subset_name_handles_empty_set() {
        let states: HashSet<StateRef> = HashSet::new();

        assert_eq!(subset_name(&states), "{}");
    }

    #[test]
    fn nfa_to_dfa_converts_simple_nfa() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let accept = nfa.add_state("accept");

        nfa.add_transition(initial.clone(), sym('a'), accept.clone());
        nfa.mark_state_accepting(accept.clone());

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
        let initial = nfa.initial().clone();

        let accept = nfa.add_state("accept");

        nfa.add_transition(initial.clone(), eps(), accept.clone());
        nfa.mark_state_accepting(accept.clone());

        let dfa = nfa.to_dfa();

        assert!(dfa.is_accepting(dfa.initial()));
        assert!(dfa.execute(""));
    }

    #[test]
    fn nfa_to_dfa_handles_nondeterminism() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let left = nfa.add_state("left");
        let right = nfa.add_state("right");
        let accept = nfa.add_state("accept");

        nfa.add_transition(initial.clone(), sym('a'), left.clone());
        nfa.add_transition(initial.clone(), sym('a'), right.clone());

        nfa.add_transition(left.clone(), sym('b'), accept.clone());
        nfa.add_transition(right.clone(), sym('c'), accept.clone());

        nfa.mark_state_accepting(accept.clone());

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
        let initial = nfa.initial().clone();

        let left = nfa.add_state("left");
        let right = nfa.add_state("right");
        let accept = nfa.add_state("accept");

        nfa.add_transition(initial.clone(), eps(), left.clone());
        nfa.add_transition(left.clone(), sym('a'), right.clone());
        nfa.add_transition(initial.clone(), sym('a'), accept.clone());

        nfa.mark_state_accepting(right.clone());
        nfa.mark_state_accepting(accept.clone());

        let dfa = nfa.to_dfa();

        assert!(dfa.execute("a"));
        assert!(!dfa.execute(""));
        assert!(!dfa.execute("aa"));
    }

    #[test]
    fn nfa_to_dfa_does_not_create_unreachable_states() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let reachable = nfa.add_state("reachable");
        let unreachable = nfa.add_state("unreachable");

        nfa.add_transition(initial, sym('a'), reachable);

        // This state is part of the NFA but cannot be reached from
        // the initial state.
        nfa.mark_state_accepting(unreachable);

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
        let initial = nfa.initial().clone();

        nfa.mark_state_accepting(initial);

        let dfa = nfa.to_dfa();

        assert!(dfa.execute(""));
        assert!(!dfa.execute("a"));
    }

    #[test]
    fn nfa_to_dfa_produces_stable_subset_names() {
        let mut nfa = NFA::new("start");
        let initial = nfa.initial().clone();

        let b = nfa.add_state("b");
        let a = nfa.add_state("a");

        nfa.add_transition(initial.clone(), sym('x'), b);
        nfa.add_transition(initial, sym('x'), a);

        let dfa = nfa.to_dfa();

        let names: HashSet<String> =
            dfa.states().map(|state| state.to_string()).collect();

        assert!(names.contains("{start}"));
        assert!(names.contains("{a,b}"));
    }

    #[test]
    fn symbol_equality_and_display() {
        assert_eq!(Symbol::epsilon().as_ref(), &Symbol::Epsilon);
        assert_eq!(Symbol::char('a').as_ref(), &Symbol::Symbol('a'));

        assert_eq!(Symbol::epsilon().to_string(), "ε");
        assert_eq!(Symbol::char('x').to_string(), "x");
    }

    #[test]
    fn state_display() {
        let state = State::new("hello");

        assert_eq!(state.to_string(), "hello");
    }
}
