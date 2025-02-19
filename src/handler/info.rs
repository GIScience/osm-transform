use std::collections::HashSet;

use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;

use crate::handler::{HandlerResult, OsmElementTypeSelection, Handler};

pub(crate) struct ElementCounter {
    pub nodes_count: u64,
    pub ways_count: u64,
    pub relations_count: u64,
    pub result_type: ElementCountResultType,
}
impl ElementCounter {
    pub fn new(result_type: ElementCountResultType) -> Self {
        Self {
            nodes_count: 0,
            ways_count: 0,
            relations_count: 0,
            result_type,
        }
    }
}
impl Handler for ElementCounter {
    fn name(&self) -> String { format!("ElementCounter {}", self.result_type.to_string()) }

    fn handle_result(&mut self, result: &mut HandlerResult) {
        match self.result_type {
            ElementCountResultType::InputCount => {
                result.input_node_count += result.nodes.len() as u64;
                result.input_way_count += result.ways.len() as u64;
                result.input_relation_count += result.relations.len() as u64;
            }
            ElementCountResultType::AcceptedCount => {
                result.accepted_node_count += result.nodes.len() as u64;
                result.accepted_way_count += result.ways.len() as u64;
                result.accepted_relation_count += result.relations.len() as u64;
            }
            ElementCountResultType::OutputCount => {
                result.output_node_count += result.nodes.len() as u64;
                result.output_way_count += result.ways.len() as u64;
                result.output_relation_count += result.relations.len() as u64;
            }
        }
    }

    // fn handle_elements(&mut self, nodes: Vec<Node>, ways: Vec<Way>, relations: Vec<Relation>) -> (Vec<Node>, Vec<Way>, Vec<Relation>) {
    //     //deactivate because new already implements handle_result
    //     (vec![], vec![], vec![])
    // }
}
#[derive(Debug)]
pub(crate) enum ElementCountResultType {
    InputCount,
    AcceptedCount,
    OutputCount,
}
impl ToString for ElementCountResultType {
    fn to_string(&self) -> String {
        match self {
            ElementCountResultType::InputCount => "InputCount".to_string(),
            ElementCountResultType::AcceptedCount => "AcceptedCount".to_string(),
            ElementCountResultType::OutputCount => "OutputCount".to_string(),
        }
    }
}

pub(crate) struct ElementPrinter {
    pub prefix: String,
    pub node_ids: HashSet<i64>,
    pub way_ids: HashSet<i64>,
    pub relation_ids: HashSet<i64>,
    pub handle_types: OsmElementTypeSelection,
}
impl Default for ElementPrinter {
    fn default() -> Self {
        Self {
            prefix: "".to_string(),
            node_ids: HashSet::new(),
            way_ids: HashSet::new(),
            relation_ids: HashSet::new(),
            handle_types: OsmElementTypeSelection::none(),
        }
    }
}
impl ElementPrinter {
    pub fn with_prefix(prefix: String) -> Self {
        Self {
            prefix: prefix,
            ..Self::default()
        }
    }
    pub(crate) fn with_node_ids(mut self, node_ids: HashSet<i64>) -> Self {
        for id in node_ids {
            self.node_ids.insert(id);
            self.handle_types.node = true;
        }
        self
    }
    pub(crate) fn with_way_ids(mut self, way_ids: HashSet<i64>) -> Self {
        for id in way_ids {
            self.way_ids.insert(id);
            self.handle_types.way = true;
        }
        self
    }
    pub(crate) fn with_relation_ids(mut self, relation_ids: HashSet<i64>) -> Self {
        for id in relation_ids {
            self.relation_ids.insert(id);
            self.handle_types.relation = true;
        }
        self
    }

    fn handle_node(&mut self, node: &Node) {
        if self.handle_types.node && self.node_ids.contains(&node.id()) {
            println!("{}: node {} visible: {}", &self.prefix, &node.id(), &node.visible());
            println!("  version:    {}", &node.version());
            println!("  coordinate: lat,lon = {},{}", &node.coordinate().lat(), &node.coordinate().lon());
            println!("  changeset:  {}", &node.changeset());
            println!("  timestamp:  {}", &node.timestamp());
            println!("  uid:        {}", &node.uid());
            println!("  user:       {}", &node.user());
            println!("  tags:");
            for tag in node.tags() {
                println!("   '{}' = '{}'", &tag.k(), &tag.v())
            }
        }
    }
    fn handle_way(&mut self, way: &Way) {
        if self.handle_types.way && self.way_ids.contains(&way.id()) {
            println!("{}: way {} visible: {}", &self.prefix, &way.id(), &way.visible());
            println!("  version:   {}", &way.version());
            println!("  changeset: {}", &way.changeset());
            println!("  timestamp: {}", &way.timestamp());
            println!("  uid:       {}", &way.uid());
            println!("  user:      {}", &way.user());
            println!("  tags:");
            for tag in way.tags() {
                println!("   '{}' = '{}'", &tag.k(), &tag.v())
            }
            println!("  refs:");
            for id in way.refs() {
                println!("   {}", &id)
            }
        }
    }
    fn handle_relation(&mut self, relation: &Relation) {
        if self.handle_types.relation && self.relation_ids.contains(&relation.id()) {
            println!("{}: relation {} visible: {}", &self.prefix, &relation.id(), &relation.visible());
            println!("  version:   {}", &relation.version());
            println!("  changeset: {}", &relation.changeset());
            println!("  timestamp: {}", &relation.timestamp());
            println!("  uid:       {}", &relation.uid());
            println!("  user:      {}", &relation.user());
            println!("  tags:");
            for tag in relation.tags() {
                println!("   '{}' = '{}'", &tag.k(), &tag.v())
            }
            println!("  members:");
            for member in relation.members() {
                println!("   {:?}", &member)
            }
        }
    }
}
impl Handler for ElementPrinter {
    fn name(&self) -> String { format!("ElementPrinter {}", self.prefix) }
    fn handle_result(&mut self, result: &mut HandlerResult) {
        self.handle_nodes(result.nodes.clone());
        self.handle_ways(result.ways.clone());
        self.handle_relations(result.relations.clone());
    }
    fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
        for node in &elements {
            self.handle_node(node);
        }
        elements
    }

    fn handle_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
        for way in &elements {
            self.handle_way(way);
        }
        elements
    }

    fn handle_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
        for relation in &elements {
            self.handle_relation(relation);
        }
        elements
    }
}

#[cfg(test)]
mod test {
    use crate::handler::info::ElementPrinter;
    use crate::handler::Handler;
    use crate::handler::tests::{simple_node};

    #[test]
    fn element_printer(){
        let mut printer = ElementPrinter::with_prefix("test".to_string()).with_node_ids( vec![1, 2].into_iter().collect() );

        // has only one bad key => should be filtered
        assert_eq!(printer.handle_nodes(vec![simple_node(1, vec![("building", "x")])]).len(), 1);
        // has only one other key => should be accepted
        assert_eq!(printer.handle_nodes(vec![simple_node(2, vec![("something", "x")])]).len(), 1);
    }

}
