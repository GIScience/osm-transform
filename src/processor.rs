use std::ops::Deref;

use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;

use crate::handler::{CountType, Handler, HandlerResult, OsmElementTypeSelection};

trait Processor {
    fn process_node(&mut self, node: Node) -> Option<Node> {
        Some(node)
    }
    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        result
    }
}

#[derive(Default)]
struct ProcessorChainExecutor {
    pub processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChainExecutor {
    fn execute(&mut self, mut node: Node) {
        for processor in &mut self.processors {
            let optional_node = processor.process_node(node);
            match optional_node {
                None => { break }
                Some(result) => { node = result }
            }
        }
    }
    fn collect_result(&mut self) -> HandlerResult{
        let mut result = HandlerResult::default();
        for processor in &mut self.processors {
            result = processor.add_result(result);
        }
        result
    }
    fn add(mut self, handler: Box<dyn Processor>) -> ProcessorChainExecutor {
        self.processors.push(handler);
        self
    }
}

struct LoggingProcessor {}
impl Processor for LoggingProcessor {
    fn process_node(&mut self, node: Node) -> Option<Node> {
        dbg!(&node);
        return Some(node);
    }
}

struct TagAdder {
    key: String,
    val: String,
}
impl Processor for TagAdder {
    fn process_node(&mut self, mut node: Node) -> Option<Node> {
        node.tags_mut().push(Tag::new(self.key.clone(), self.val.clone()));
        return Some(node);
    }
}


pub(crate) struct Counter {
    pub nodes_count: i32,
    pub ways_count: i32,
    pub relations_count: i32,
    pub handle_types: OsmElementTypeSelection,
    pub count_type: CountType,
}
impl Counter {
    pub fn new(handle_types: OsmElementTypeSelection, count_type: CountType) -> Self {
        Self {
            nodes_count: 0,
            ways_count: 0,
            relations_count: 0,
            handle_types,
            count_type,
        }
    }
}
impl Processor for Counter {
    fn process_node(&mut self, node: Node) -> Option<Node> {
        if self.handle_types.node {
            self.nodes_count += 1;
        }
        Some(node)
    }
    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        match self.count_type {
            CountType::ALL => { result.count_all_nodes = self.nodes_count }
            CountType::ACCEPTED => { result.count_accepted_nodes = self.nodes_count }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use simple_logger::SimpleLogger;

    use crate::handler::{CountType, OsmElementTypeSelection};
    use crate::processor::{Counter, LoggingProcessor, Processor, ProcessorChainExecutor, TagAdder};

    #[test]
    fn executor_add_preprocessors_vector() {
        SimpleLogger::new().init();
        let mut processors: Vec<Box<dyn Processor>> = vec![
            Box::new(LoggingProcessor {}),
            Box::new(Counter::new(OsmElementTypeSelection::node_only(), CountType::ALL)),
            Box::new(TagAdder { key: "k1".to_string(), val: "v1".to_string() }),
            Box::new(LoggingProcessor {}),
        ];
        let mut executor = ProcessorChainExecutor { processors };

        executor.execute(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "kasper".to_string())]));
        executor.execute(Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "seppl".to_string())]));
        executor.execute(Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "hotzenplotz".to_string())]));
        executor.execute(Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "großmutter".to_string())]));
    }

    #[test]
    fn executor_add_preprocessors_fluent() {
        SimpleLogger::new().init();
        let mut executor = ProcessorChainExecutor::default()
            .add(Box::new(LoggingProcessor {}))
            .add(Box::new(Counter::new(OsmElementTypeSelection::node_only(), CountType::ALL)))
            .add(Box::new(TagAdder { key: "k1".to_string(), val: "v1".to_string() }))
            .add(Box::new(LoggingProcessor {}));

        executor.execute(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "kasper".to_string())]));
        executor.execute(Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "seppl".to_string())]));
        executor.execute(Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "hotzenplotz".to_string())]));
        executor.execute(Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "großmutter".to_string())]));
    }


    #[test]
    fn executor_collect_result() {
        SimpleLogger::new().init();
        let mut executor = ProcessorChainExecutor::default()
            .add(Box::new(Counter::new(OsmElementTypeSelection::node_only(), CountType::ALL)))
            .add(Box::new(Counter::new(OsmElementTypeSelection::node_only(), CountType::ACCEPTED)));

        executor.execute(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "kasper".to_string())]));
        executor.execute(Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "seppl".to_string())]));
        executor.execute(Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "hotzenplotz".to_string())]));
        executor.execute(Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new("person".to_string(), "großmutter".to_string())]));
        let result = executor.collect_result();
        dbg!(&result);
        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 4);
    }
}