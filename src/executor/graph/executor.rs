use crate::command::Command;
use crate::config::Config;
use crate::executor::graph::DependencyGraph;
use crate::executor::{ExecutionInfoKey, Executor, ExecutorResult};
use crate::id::{Dot, ProcessId, Rifl};
use crate::kvs::KVStore;
use std::collections::HashSet;
use threshold::VClock;

pub struct GraphExecutor {
    graph: DependencyGraph,
    store: KVStore,
    pending: HashSet<Rifl>,
}

impl Executor for GraphExecutor {
    type ExecutionInfo = GraphExecutionInfo;

    fn new(config: Config) -> Self {
        // this executor can never be parallel
        assert!(!config.parallel_executor());
        let graph = DependencyGraph::new(&config);
        let store = KVStore::new();
        let pending = HashSet::new();
        Self {
            graph,
            store,
            pending,
        }
    }

    fn register(&mut self, rifl: Rifl, _key_count: usize) {
        // start command in pending
        assert!(self.pending.insert(rifl));
    }

    fn handle(&mut self, info: Self::ExecutionInfo) -> Vec<ExecutorResult> {
        // borrow everything we'll need
        let graph = &mut self.graph;
        let store = &mut self.store;
        let pending = &mut self.pending;

        // handle each new info
        graph.add(info.dot, info.cmd, info.clock);

        // get more commands that are ready to be executed
        let to_execute = graph.commands_to_execute();

        // execute them all
        to_execute
            .into_iter()
            .filter_map(|cmd| {
                // get command rifl
                let rifl = cmd.rifl();
                // execute the command
                let result = cmd.execute(store);

                // if it was pending locally, then it's from a client of this process
                if pending.remove(&rifl) {
                    Some(ExecutorResult::Ready(result))
                } else {
                    None
                }
            })
            .collect()
    }

    fn parallel(&self) -> bool {
        false
    }

    fn show_metrics(&self) {
        self.graph.show_metrics();
    }
}

#[derive(Debug, Clone)]
pub struct GraphExecutionInfo {
    dot: Dot,
    cmd: Command,
    clock: VClock<ProcessId>,
}

impl GraphExecutionInfo {
    pub fn new(dot: Dot, cmd: Command, clock: VClock<ProcessId>) -> Self {
        Self { dot, cmd, clock }
    }
}

impl ExecutionInfoKey for GraphExecutionInfo {}
