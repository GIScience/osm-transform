use bit_vec::BitVec;
use osm_io::osm::model::relation::{Member, Relation};
use osm_io::osm::model::way::Way;

use crate::handler::{HandlerResult, HIGHEST_NODE_ID, Handler};

pub(crate) struct ReferencedNodeIdCollector {
    referenced_node_ids: BitVec,
    count_unique: usize,
}
impl ReferencedNodeIdCollector {
    pub(crate) fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }

    fn with_capacity(nbits: usize) -> Self {
        ReferencedNodeIdCollector {
            referenced_node_ids: BitVec::from_elem(nbits, false),
            count_unique: 0
        }
    }

    fn add_node_id(&mut self, id: i64) {
        if matches!(self.referenced_node_ids.get(id as usize), Some(false)) { self.count_unique += 1 }
        self.referenced_node_ids.set(id as usize, true);
    }
}
impl Handler for ReferencedNodeIdCollector {
    fn name(&self) -> String {
        "ReferencedNodeIdCollector".to_string()
    }

    fn handle_ways(&mut self, mut elements: Vec<Way>) -> Vec<Way> {
        for element in &mut *elements {
            for &id in element.refs() {
                self.add_node_id(id);
            }
        }
        elements
    }

    fn handle_relations(&mut self, mut elements: Vec<Relation>) -> Vec<Relation> {
        for element in &mut *elements {
            for member in element.members() {
                match member {
                    Member::Node { member } => {
                        self.add_node_id(member.id());
                    }
                    Member::Way { .. } => {}
                    Member::Relation { .. } => {}
                }
            }
        }
        elements
    }

    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        log::debug!("cloning node_ids of ReferencedNodeIdCollector with len={} into HandlerResult ", self.referenced_node_ids.len());
        result.node_ids = self.referenced_node_ids.clone();//todo check if clone is necessary
        result
    }
}

#[cfg(test)]
mod test {
    use crate::handler::{HIGHEST_NODE_ID, Handler};
    use crate::handler::tests::{simple_node, TestOnlyIdCollector};

    #[test]
    fn node_id_collector(){
        let mut collector = TestOnlyIdCollector::new(10);
        assert_eq!(10, collector.node_ids.len());
        collector.handle_nodes(vec![simple_node(2, vec![])]);
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(2).unwrap_or(false));
    }
    #[test]
    #[should_panic(expected = "index out of bounds: 12 >= 10")]
    fn node_id_collector_out_of_bounds(){
        let mut collector = TestOnlyIdCollector::new(10);
        collector.handle_nodes(vec![simple_node(12,vec![])]);
    }
    #[test]
    fn node_id_collector_out_of_bounds_real(){
        let mut collector = TestOnlyIdCollector::new(HIGHEST_NODE_ID as usize);

        collector.handle_nodes(vec![simple_node(1, vec![])]);
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(2).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(11414456780).unwrap_or(false));

        collector.handle_nodes(vec![simple_node(11414456780, vec![])]);
        assert_eq!(true, collector.node_ids.get(11414456780).unwrap_or(false));
    }

}